use std::{path::PathBuf, sync::Arc};
use worktable_server::WebState;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("项目路径无效")?
        .to_path_buf();
    // A separate web data directory prevents concurrent desktop notification/VPN ownership.
    let data = std::env::var_os("WORKTABLE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::data_dir().map(|p| p.join("org.hyksj.worktable-web")))
        .ok_or("无法确定用户数据目录")?;
    let state = Arc::new(WebState::open(&data, root.join("src-tauri"))?);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 1421))
        .await
        .map_err(|_| "端口 1421 已占用，请先关闭其他 Worktable 网页后端。")?;
    println!("Worktable 后端：http://127.0.0.1:1421 · 仅本机访问");
    let shutdown = Arc::clone(&state);
    axum::serve(listener, worktable_server::router(state, root.join("dist")))
        .with_graceful_shutdown(async move {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("SIGTERM handler");
                tokio::select! {_ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {}}
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            let _ = tokio::task::spawn_blocking(move || shutdown.shutdown()).await;
        })
        .await?;
    Ok(())
}
