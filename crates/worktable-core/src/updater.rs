use crate::validation::{self, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    version: String,
    release_notes: String,
    downloads: HashMap<String, String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    current_version: String,
    version: String,
    release_notes: String,
    download_url: Option<String>,
    available: bool,
}
pub async fn check(endpoint: &str) -> AppResult<UpdateInfo> {
    if endpoint.is_empty() {
        return Err("尚未配置版本检查地址。请在设置中填写管理员提供的 HTTPS endpoint。".into());
    }
    if validation::web_url(endpoint)?.scheme() != "https" {
        return Err("版本检查必须使用 HTTPS。".into());
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| "无法初始化版本检查。")?;
    let mut response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|_| "版本服务不可访问，请稍后重试。")?
        .error_for_status()
        .map_err(|_| "版本服务返回错误。")?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "无法读取版本信息。")? {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err("版本信息过大。".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let m: Manifest = serde_json::from_slice(&bytes).map_err(|_| "版本信息格式无效。")?;
    let current = env!("CARGO_PKG_VERSION");
    let new = semver::Version::parse(&m.version).map_err(|_| "版本号无效。")?;
    let key = if cfg!(windows) {
        "windows-x64"
    } else if cfg!(target_os = "macos") {
        "macos-arm64"
    } else {
        "linux-x64"
    };
    let download = m.downloads.get(key).cloned();
    if let Some(url) = &download {
        if validation::web_url(url)?.scheme() != "https" {
            return Err("下载链接必须使用 HTTPS。".into());
        }
    }
    Ok(UpdateInfo {
        current_version: current.into(),
        version: m.version,
        release_notes: m.release_notes,
        download_url: download,
        available: new > semver::Version::parse(current).map_err(|_| "应用版本无效。")?,
    })
}
