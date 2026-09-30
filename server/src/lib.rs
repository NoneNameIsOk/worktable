use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::services::ServeDir;
use worktable_core::{
    db::Database,
    model::*,
    platform, updater,
    validation::{self, AppResult},
    vpn::{profile::VpnProfileManager, BundledOpenVpnBackend, VpnBackend, VpnManager, VpnState},
};

pub struct WebState {
    db: Mutex<Database>,
    profiles: VpnProfileManager,
    vpn: VpnManager,
    backend: BundledOpenVpnBackend,
}
impl WebState {
    pub fn open(data: &std::path::Path, resources: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(data).map_err(|_| "无法创建本地数据目录。")?;
        worktable_core::vpn::profile::private_permissions(data, true)?;
        Ok(Self {
            db: Mutex::new(Database::open(&data.join("worktable.sqlite3"))?),
            profiles: VpnProfileManager::new(data)?,
            vpn: VpnManager::default(),
            backend: BundledOpenVpnBackend {
                resource_dir: resources,
            },
        })
    }
    fn db(&self) -> AppResult<std::sync::MutexGuard<'_, Database>> {
        self.db.lock().map_err(|_| "本地数据库暂时不可用。".into())
    }
    pub fn shutdown(&self) {
        self.vpn.shutdown();
    }
    fn connect(&self, id: &str) -> AppResult<()> {
        if !self
            .db()?
            .snapshot()?
            .vpn_profiles
            .iter()
            .any(|p| p.id == id)
        {
            return Err("VPN 配置不存在。".into());
        }
        let path = self.profiles.path(id)?;
        let content = std::fs::read_to_string(&path).map_err(|_| "无法读取私有 VPN 配置。")?;
        worktable_core::vpn::profile::preflight(&content)?;
        let password = uuid::Uuid::new_v4().to_string();
        let secret_file = self.profiles.password_file(&password)?;
        self.vpn
            .connect(&self.backend, id.into(), path, secret_file, password)?;
        self.db()?.set_last_profile(id)
    }
    async fn health(&self) -> AppResult<bool> {
        let url = self.db()?.settings()?.cluster_health_check_url;
        validation::web_url(&url)?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|_| "无法初始化网络检测。")?;
        Ok(client
            .get(url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success()))
    }
    async fn require_network(&self) -> AppResult<()> {
        if self.vpn.status()?.state == VpnState::Connected || self.health().await? {
            return Ok(());
        }
        Err("实验室网络尚不可达。请通过 VPN 连接后重试；若已连接，请检查集群检测地址。".into())
    }
}

#[derive(Debug)]
struct ApiError(StatusCode, String);
impl From<String> for ApiError {
    fn from(s: String) -> Self {
        Self(StatusCode::BAD_REQUEST, s)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}
fn parse<T: DeserializeOwned>(value: Value) -> AppResult<T> {
    serde_json::from_value(value).map_err(|_| "请求格式无效，请检查字段和类型。".into())
}
fn value<T: serde::Serialize>(v: T) -> AppResult<Value> {
    serde_json::to_value(v).map_err(|_| "无法生成响应。".into())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileId {
    profile_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NodeId {
    node_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServiceId {
    service_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Upload {
    name: String,
    content: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutateRequest {
    mutation: Mutation,
}

async fn command(
    State(state): State<Arc<WebState>>,
    Path(command): Path<String>,
    body: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(args) = body.map_err(|_| {
        ApiError(
            StatusCode::BAD_REQUEST,
            "请求必须为有效 JSON，且不能超过 6 MB。".into(),
        )
    })?;
    Ok(Json(dispatch(&state, &command, args).await?))
}
async fn dispatch(state: &Arc<WebState>, command: &str, args: Value) -> AppResult<Value> {
    match command {
        "get_snapshot" => value(state.db()?.snapshot()?),
        "mutate" => {
            let mut m: MutateRequest = parse(args)?;
            // Web settings cannot silently change OS registration or desktop startup behavior.
            if let Mutation::SaveSettings(s) = &mut m.mutation {
                let old = state.db()?.settings()?;
                if s.autostart != old.autostart
                    || s.start_in_tray != old.start_in_tray
                    || s.vpn_autoconnect != old.vpn_autoconnect
                {
                    return Err("网页版不提供桌面启动设置，请保留原值。".into());
                }
            }
            let mut db = state.db()?;
            db.mutate(m.mutation)?;
            value(db.snapshot()?)
        }
        "vpn_status" => value(state.vpn.status()?),
        "vpn_capability" => value(state.backend.capability()),
        "vpn_connect" => {
            let p: ProfileId = parse(args)?;
            state.connect(&p.profile_id)?;
            Ok(Value::Null)
        }
        "vpn_disconnect" => {
            state.vpn.disconnect()?;
            Ok(Value::Null)
        }
        "import_vpn_profile" => {
            let upload: Upload = parse(args)?;
            if !upload.name.to_lowercase().ends_with(".ovpn") {
                return Err("请选择 .ovpn 文件。".into());
            }
            let name = upload.name.strip_suffix(".ovpn").unwrap_or(&upload.name);
            let profile = state.profiles.import_content(name, &upload.content)?;
            if let Err(e) = state.db()?.add_profile(&profile) {
                let _ = state.profiles.remove(&profile.id);
                return Err(e);
            }
            Ok(json!(true))
        }
        "delete_vpn_profile" => {
            let p: ProfileId = parse(args)?;
            let status = state.vpn.status()?;
            if status.profile_id.as_ref() == Some(&p.profile_id)
                && !matches!(status.state, VpnState::Disconnected | VpnState::Failed)
            {
                return Err("请先断开此 VPN，再删除配置。".into());
            }
            state.profiles.remove(&p.profile_id)?;
            state.db()?.delete_profile(&p.profile_id)?;
            Ok(Value::Null)
        }
        "validate_lab_config" => {
            let c: LabConfig = parse(args)?;
            validation::lab_config(&c)?;
            value(c)
        }
        "import_lab_config" => {
            let c: LabConfig = parse(args)?;
            state.db()?.import_lab(c)?;
            Ok(json!(true))
        }
        "export_lab_config" => value(state.db()?.export_lab()?),
        "cluster_health" => value(state.health().await?),
        "service_url" => {
            let p: ServiceId = parse(args)?;
            let s = state
                .db()?
                .snapshot()?
                .services
                .into_iter()
                .find(|s| s.id == p.service_id && s.enabled)
                .ok_or("服务不存在或已禁用。")?;
            if s.requires_vpn {
                state.require_network().await?;
            }
            validation::web_url(&s.url)?;
            value(s.url)
        }
        "open_node" | "probe_node" => {
            let p: NodeId = parse(args)?;
            let n = state
                .db()?
                .snapshot()?
                .nodes
                .into_iter()
                .find(|n| n.id == p.node_id && n.enabled)
                .ok_or("节点不存在或已禁用。")?;
            validation::ssh_host(&n.ssh_host)?;
            state.require_network().await?;
            if command == "open_node" {
                tokio::task::spawn_blocking(move || platform::open_vscode(&n.ssh_host))
                    .await
                    .map_err(|_| "VS Code 启动任务失败。")??;
                Ok(Value::Null)
            } else {
                let reachable = tokio::task::spawn_blocking(move || {
                    use std::net::ToSocketAddrs;
                    let mut addresses = (n.ssh_host.as_str(), 22).to_socket_addrs().map_err(|_| "无法解析节点。SSH config 别名不一定能通过 DNS 检测，请用 VS Code 连接。")?;
                    Ok::<_, String>(addresses.by_ref().take(4).any(|a| std::net::TcpStream::connect_timeout(&a, Duration::from_secs(2)).is_ok()))
                }).await.map_err(|_| "节点检测任务失败。")??;
                value(reachable)
            }
        }
        "claim_reminders" => value(state.db()?.claim_reminders()?),
        "check_update" => {
            let endpoint = state.db()?.settings()?.update_endpoint;
            value(updater::check(&endpoint).await?)
        }
        _ => Err("此操作不适用于网页版。".into()),
    }
}

// A local privileged API must reject cross-origin browser requests and DNS rebinding.
// There is deliberately no CORS allowlist, and all API reads also require this header.
fn trusted(headers: &HeaderMap) -> bool {
    let allowed_hosts = [
        "127.0.0.1:1420",
        "localhost:1420",
        "127.0.0.1:1421",
        "localhost:1421",
    ];
    let host = headers.get("host").and_then(|h| h.to_str().ok());
    let origin = headers.get("origin").and_then(|h| h.to_str().ok());
    let origin_ok = origin.is_none_or(|o| allowed_hosts.iter().any(|h| o == format!("http://{h}")));
    host.is_some_and(|h| allowed_hosts.contains(&h))
        && origin_ok
        && headers
            .get("x-worktable-client")
            .is_some_and(|h| h == "web")
        && headers
            .get("sec-fetch-site")
            .is_none_or(|h| h != "cross-site")
}
async fn guard(req: Request, next: Next) -> Response {
    if !trusted(req.headers()) {
        return ApiError(
            StatusCode::FORBIDDEN,
            "拒绝来自其他网站的本地接口调用。".into(),
        )
        .into_response();
    }
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().expect("static header"));
    response
}
async fn web_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    headers.insert("x-frame-options", "DENY".parse().expect("static header"));
    headers.insert(
        "referrer-policy",
        "no-referrer".parse().expect("static header"),
    );
    headers.insert("content-security-policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; object-src 'none'; base-uri 'self'".parse().expect("static header"));
    response
}
pub fn router(state: Arc<WebState>, dist: PathBuf) -> Router {
    let api = Router::new()
        .route("/{command}", post(command))
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
        .layer(middleware::from_fn(guard))
        .with_state(state);
    Router::new()
        .nest("/api", api)
        .fallback_service(ServeDir::new(dist))
        .layer(middleware::from_fn(web_headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    fn state(root: &std::path::Path) -> Arc<WebState> {
        Arc::new(WebState::open(root, root.into()).unwrap())
    }
    async fn call(
        app: &Router,
        name: &str,
        args: Value,
        origin: &str,
        header: bool,
    ) -> (StatusCode, Value) {
        let mut b = Request::builder()
            .method("POST")
            .uri(format!("/api/{name}"))
            .header("host", "127.0.0.1:1421")
            .header("origin", origin)
            .header("content-type", "application/json");
        if header {
            b = b.header("x-worktable-client", "web");
        }
        let res = app
            .clone()
            .oneshot(b.body(Body::from(args.to_string())).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = to_bytes(res.into_body(), 8 * 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    const ORIGIN: &str = "http://127.0.0.1:1420";
    #[tokio::test]
    async fn rejects_cross_site_and_missing_header() {
        let root = tempfile::tempdir().unwrap();
        let app = router(state(root.path()), root.path().into());
        assert_eq!(
            call(
                &app,
                "get_snapshot",
                json!({}),
                "https://evil.example",
                true
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(&app, "get_snapshot", json!({}), ORIGIN, false).await.0,
            StatusCode::FORBIDDEN
        );
        let mut h = HeaderMap::new();
        h.insert("host", "evil.example:1421".parse().unwrap());
        h.insert("x-worktable-client", "web".parse().unwrap());
        assert!(!trusted(&h));
    }
    #[tokio::test]
    async fn api_persists_after_reopening_database() {
        let root = tempfile::tempdir().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        {
            let app = router(state(root.path()), root.path().into());
            let (status, body)=call(&app,"mutate",json!({"mutation":{"type":"saveTodo","value":{"id":id,"title":"网页待办","completed":false,"dueDate":null}}}),ORIGIN,true).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["todos"][0]["title"], "网页待办");
        }
        let app = router(state(root.path()), root.path().into());
        let (_, body) = call(&app, "get_snapshot", json!({}), ORIGIN, true).await;
        assert_eq!(body["todos"][0]["id"], id);
    }
    #[tokio::test]
    async fn uploads_private_profile_and_exports_only_public_data() {
        let root = tempfile::tempdir().unwrap();
        let app = router(state(root.path()), root.path().into());
        let config = "client\nremote vpn.example.org 1194\n<ca>\nTEST ONLY\n</ca>\n";
        assert_eq!(
            call(
                &app,
                "import_vpn_profile",
                json!({"name":"test.ovpn","content":config}),
                ORIGIN,
                true
            )
            .await
            .0,
            StatusCode::OK
        );
        let (_, s) = call(&app, "get_snapshot", json!({}), ORIGIN, true).await;
        assert_eq!(s["vpnProfiles"].as_array().unwrap().len(), 1);
        assert!(!s.to_string().contains("TEST ONLY"));
        let (_, public) = call(&app, "export_lab_config", json!({}), ORIGIN, true).await;
        assert!(public.get("vpnProfiles").is_none());
        assert!(public.get("todos").is_none());
        let mut bad = public.clone();
        bad["schemaVersion"] = json!(2);
        assert_eq!(
            call(&app, "import_lab_config", bad, ORIGIN, true).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(&app, "import_lab_config", public, ORIGIN, true)
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            call(
                &app,
                "import_vpn_profile",
                json!({"name":"bad.ovpn","content":format!("{config}up /tmp/bad")}),
                ORIGIN,
                true
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    #[tokio::test]
    async fn cannot_enable_desktop_startup_from_web() {
        let root = tempfile::tempdir().unwrap();
        let app = router(state(root.path()), root.path().into());
        let (_, snap) = call(&app, "get_snapshot", json!({}), ORIGIN, true).await;
        let mut settings = snap["settings"].clone();
        settings["autostart"] = json!(true);
        assert_eq!(
            call(
                &app,
                "mutate",
                json!({"mutation":{"type":"saveSettings","value":settings}}),
                ORIGIN,
                true
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}
