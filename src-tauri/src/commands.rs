use crate::{
    model::*,
    validation::{self, AppResult},
    vpn::{VpnBackend, VpnState},
    web_services::Bounds,
    AppState,
};
use tauri::{Manager, State, Webview};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
// Capabilities restrict plugin APIs. This additional check covers every custom
// command, including commands Tauri otherwise allows by default.
fn local(w: &Webview) -> AppResult<()> {
    let url = w.url().map_err(|_| "无法验证调用来源。")?;
    let trusted = url.scheme() == "tauri" && url.host_str() == Some("localhost")
        || matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost")
        || cfg!(debug_assertions)
            && url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(1420);
    if w.label() != "main" || !trusted {
        return Err("此页面无权访问 Worktable 本地能力。".into());
    }
    Ok(())
}
#[tauri::command]
pub fn get_snapshot(webview: Webview, state: State<'_, AppState>) -> AppResult<Snapshot> {
    local(&webview)?;
    state.db()?.snapshot()
}
#[tauri::command]
pub fn mutate(
    webview: Webview,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mutation: Mutation,
) -> AppResult<Snapshot> {
    local(&webview)?;
    if let Mutation::SaveSettings(s) = &mutation {
        validation::settings(s)?;
        let old = state.db()?.settings()?;
        if old.autostart != s.autostart {
            let manager = app.autolaunch();
            if s.autostart {
                manager.enable()
            } else {
                manager.disable()
            }
            .map_err(|_| "无法修改系统开机启动设置，请检查系统权限。")?;
            if let Err(e) = state.db()?.mutate(mutation) {
                let _ = if old.autostart {
                    manager.enable()
                } else {
                    manager.disable()
                };
                return Err(e);
            }
            return state.db()?.snapshot();
        }
    }
    let close_id = match &mutation {
        Mutation::SaveService(s) => {
            let old = state
                .db()?
                .snapshot()?
                .services
                .into_iter()
                .find(|old| old.id == s.id);
            old.filter(|old| {
                old.url != s.url || old.enabled != s.enabled || old.requires_vpn != s.requires_vpn
            })
            .map(|_| s.id.clone())
        }
        Mutation::DeleteService(id) => Some(id.clone()),
        _ => None,
    };
    state.db()?.mutate(mutation)?;
    if let Some(id) = close_id {
        state
            .services
            .lock()
            .map_err(|_| "服务视图繁忙。")?
            .close(&app, &id)?;
    }
    state.db()?.snapshot()
}
#[tauri::command]
pub fn vpn_status(
    webview: Webview,
    state: State<'_, AppState>,
) -> AppResult<crate::vpn::VpnStatus> {
    local(&webview)?;
    state.vpn.status()
}
#[tauri::command]
pub fn vpn_capability(
    webview: Webview,
    state: State<'_, AppState>,
) -> AppResult<crate::platform::Capability> {
    local(&webview)?;
    Ok(state.backend.capability())
}
pub fn connect_profile(state: &AppState, id: &str) -> AppResult<()> {
    let exists = state
        .db()?
        .snapshot()?
        .vpn_profiles
        .iter()
        .any(|p| p.id == id);
    if !exists {
        return Err("VPN 配置不存在，请重新选择。".into());
    }
    let path = state.profiles.path(id)?;
    let content = std::fs::read_to_string(&path).map_err(|_| "无法读取私有 VPN 配置。")?;
    crate::vpn::profile::preflight(&content)?;
    let secret = uuid::Uuid::new_v4().to_string();
    let pass = state.profiles.password_file(&secret)?;
    state
        .vpn
        .connect(&state.backend, id.into(), path, pass, secret)?;
    state.db()?.set_last_profile(id)
}
#[tauri::command]
pub fn vpn_connect(
    webview: Webview,
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<()> {
    local(&webview)?;
    connect_profile(&state, &profile_id)
}
#[tauri::command]
pub fn vpn_disconnect(webview: Webview, state: State<'_, AppState>) -> AppResult<()> {
    local(&webview)?;
    state.vpn.disconnect()
}
#[tauri::command]
pub async fn import_vpn_profile(webview: Webview, app: tauri::AppHandle) -> AppResult<bool> {
    local(&webview)?;
    let file = rfd::AsyncFileDialog::new()
        .add_filter("OpenVPN", &["ovpn"])
        .pick_file()
        .await;
    let Some(file) = file else { return Ok(false) };
    let state = app.state::<AppState>();
    let p = state.profiles.import(file.path())?;
    if let Err(e) = state.db()?.add_profile(&p) {
        let _ = state.profiles.remove(&p.id);
        return Err(e);
    }
    Ok(true)
}
#[tauri::command]
pub fn delete_vpn_profile(
    webview: Webview,
    state: State<'_, AppState>,
    profile_id: String,
) -> AppResult<()> {
    local(&webview)?;
    let s = state.vpn.status()?;
    if s.profile_id.as_ref() == Some(&profile_id)
        && !matches!(s.state, VpnState::Disconnected | VpnState::Failed)
    {
        return Err("请先断开此 VPN，再删除配置。".into());
    }
    state.profiles.remove(&profile_id)?;
    state.db()?.delete_profile(&profile_id)
}
#[tauri::command]
pub async fn import_lab_config(webview: Webview, app: tauri::AppHandle) -> AppResult<bool> {
    local(&webview)?;
    let file = rfd::AsyncFileDialog::new()
        .add_filter("实验室配置", &["json"])
        .pick_file()
        .await;
    let Some(file) = file else { return Ok(false) };
    let path = file.path().canonicalize().map_err(|_| "无法读取配置包。")?;
    let metadata = std::fs::metadata(&path).map_err(|_| "无法读取配置包。")?;
    if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
        return Err("配置包必须是小于 2 MB 的 JSON 文件。".into());
    }
    let content = std::fs::read(path).map_err(|_| "无法读取配置包。")?;
    let c: LabConfig = serde_json::from_slice(&content)
        .map_err(|_| "配置包不符合 schema：请检查字段、类型及是否包含非公共字段。")?;
    validation::lab_config(&c)?;
    let confirmed=rfd::AsyncMessageDialog::new().set_title("导入实验室配置").set_description(format!("将替换当前服务和节点列表，导入「{}」的 {} 个服务和 {} 个节点。Todo、日程和 VPN 配置保留。",c.lab_name,c.services.len(),c.nodes.len())).set_buttons(rfd::MessageButtons::OkCancel).show().await;
    if confirmed != rfd::MessageDialogResult::Ok {
        return Ok(false);
    }
    let state = app.state::<AppState>();
    state.db()?.import_lab(c)?;
    state
        .services
        .lock()
        .map_err(|_| "服务视图繁忙。")?
        .close_all(&app)?;
    Ok(true)
}
#[tauri::command]
pub async fn export_lab_config(webview: Webview, app: tauri::AppHandle) -> AppResult<bool> {
    local(&webview)?;
    let config = app.state::<AppState>().db()?.export_lab()?;
    let text = serde_json::to_vec_pretty(&config).map_err(|_| "配置导出失败。")?;
    let file = rfd::AsyncFileDialog::new()
        .add_filter("实验室配置", &["json"])
        .set_file_name("lab-config.json")
        .save_file()
        .await;
    let Some(file) = file else { return Ok(false) };
    std::fs::write(file.path(), text).map_err(|_| "无法保存配置包，请检查目标目录权限。")?;
    Ok(true)
}
#[tauri::command]
pub async fn show_service(
    webview: Webview,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    service_id: String,
    bounds: Bounds,
) -> AppResult<()> {
    local(&webview)?;
    let s = state
        .db()?
        .snapshot()?
        .services
        .into_iter()
        .find(|s| s.id == service_id)
        .ok_or("服务不存在。")?;
    if !s.enabled {
        return Err("此服务已禁用。".into());
    }
    if s.requires_vpn && state.vpn.status()?.state != VpnState::Connected {
        return Err("此服务需要 VPN，请先连接实验室网络。".into());
    }
    state
        .services
        .lock()
        .map_err(|_| "服务视图繁忙。")?
        .show(&app, &s, &bounds)
}
#[tauri::command]
pub fn hide_services(
    webview: Webview,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    local(&webview)?;
    state
        .services
        .lock()
        .map_err(|_| "服务视图繁忙。")?
        .hide(&app)
}
#[tauri::command]
pub fn resize_service(
    webview: Webview,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    bounds: Bounds,
) -> AppResult<()> {
    local(&webview)?;
    state
        .services
        .lock()
        .map_err(|_| "服务视图繁忙。")?
        .resize(&app, &bounds)
}
#[tauri::command]
pub fn reset_service(
    webview: Webview,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    service_id: String,
) -> AppResult<()> {
    local(&webview)?;
    state
        .services
        .lock()
        .map_err(|_| "服务视图繁忙。")?
        .close(&app, &service_id)
}
#[tauri::command]
pub async fn open_node(webview: Webview, app: tauri::AppHandle, node_id: String) -> AppResult<()> {
    local(&webview)?;
    let host = {
        let state = app.state::<AppState>();
        if state.vpn.status()?.state != VpnState::Connected {
            return Err("节点需要 VPN，请先连接实验室网络。".into());
        }
        let n = state
            .db()?
            .snapshot()?
            .nodes
            .into_iter()
            .find(|n| n.id == node_id && n.enabled)
            .ok_or("节点不存在或已禁用。")?;
        n.ssh_host
    };
    tauri::async_runtime::spawn_blocking(move || crate::platform::open_vscode(&host))
        .await
        .map_err(|_| "VS Code 启动任务失败。")?
}
#[tauri::command]
pub async fn probe_node(
    webview: Webview,
    app: tauri::AppHandle,
    node_id: String,
) -> AppResult<bool> {
    local(&webview)?;
    let host = {
        let state = app.state::<AppState>();
        if state.vpn.status()?.state != VpnState::Connected {
            return Err("请先连接 VPN 再检测节点。".into());
        }
        let host = state
            .db()?
            .snapshot()?
            .nodes
            .into_iter()
            .find(|n| n.id == node_id && n.enabled)
            .ok_or("节点不存在或已禁用。")?
            .ssh_host;
        host
    };
    tauri::async_runtime::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        let addresses = (host.as_str(), 22).to_socket_addrs().map_err(|_| {
            "无法解析节点主机名。SSH config 别名可能无法通过系统 DNS 解析，但仍可用 VS Code 连接。"
        })?;
        Ok(addresses.take(4).any(|a| {
            std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(2)).is_ok()
        }))
    })
    .await
    .map_err(|_| "节点检测任务失败。")?
}
#[tauri::command]
pub async fn cluster_health(webview: Webview, app: tauri::AppHandle) -> AppResult<bool> {
    local(&webview)?;
    let url = app
        .state::<AppState>()
        .db()?
        .settings()?
        .cluster_health_check_url;
    validation::web_url(&url)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法初始化连通性检测。")?;
    Ok(client
        .get(url)
        .send()
        .await
        .is_ok_and(|r| r.status().is_success()))
}
#[tauri::command]
pub fn request_notifications(webview: Webview, app: tauri::AppHandle) -> AppResult<String> {
    local(&webview)?;
    app.notification()
        .request_permission()
        .map(|s| format!("{s:?}"))
        .map_err(|_| "无法请求通知权限，请在系统设置中允许 Worktable 通知。".into())
}
#[tauri::command]
pub async fn check_update(
    webview: Webview,
    app: tauri::AppHandle,
) -> AppResult<crate::updater::UpdateInfo> {
    local(&webview)?;
    let endpoint = app.state::<AppState>().db()?.settings()?.update_endpoint;
    crate::updater::check(&endpoint).await
}
#[tauri::command]
pub fn open_download(webview: Webview, url: String) -> AppResult<()> {
    local(&webview)?;
    if validation::web_url(&url)?.scheme() != "https" {
        return Err("下载链接必须使用 HTTPS。".into());
    }
    crate::platform::open_url(&url)
}
