use crate::{
    model::Service,
    validation::{self, AppResult},
};
use serde::Deserialize;
use std::collections::HashSet;
use tauri::{webview::WebviewBuilder, LogicalPosition, LogicalSize, Manager, WebviewUrl};
#[derive(Default)]
pub struct ServiceViews {
    pub labels: HashSet<String>,
    pub active: Option<String>,
}
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Bounds {
    pub fn validate(&self) -> AppResult<()> {
        if ![self.x, self.y, self.width, self.height]
            .iter()
            .all(|x| x.is_finite())
            || self.x < 200.0
            || self.y < 80.0
            || self.width < 1.0
            || self.height < 1.0
        {
            return Err("服务视图尺寸无效。".into());
        }
        Ok(())
    }
}
// DOM coordinates are relative to the main webview, which may have a native
// titlebar/content offset. Convert to the parent window's logical coordinates.
fn position(app: &tauri::AppHandle, b: &Bounds) -> AppResult<LogicalPosition<f64>> {
    let main = app.get_webview("main").ok_or("主视图不可用。")?;
    let offset = main.position().map_err(|_| "无法读取主视图位置。")?;
    let scale = main
        .window()
        .scale_factor()
        .map_err(|_| "无法读取窗口缩放比例。")?;
    Ok(LogicalPosition::new(
        b.x + f64::from(offset.x) / scale,
        b.y + f64::from(offset.y) / scale,
    ))
}
impl ServiceViews {
    pub fn hide(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        for label in &self.labels {
            if let Some(w) = app.get_webview(label) {
                w.hide().map_err(|_| "无法隐藏服务视图。")?;
            }
        }
        self.active = None;
        Ok(())
    }
    pub fn show(&mut self, app: &tauri::AppHandle, s: &Service, b: &Bounds) -> AppResult<()> {
        b.validate()?;
        self.hide(app)?;
        let label = format!("service-{}", s.id);
        if !self.labels.contains(&label) {
            let url = validation::web_url(&s.url)?;
            let builder = WebviewBuilder::new(&label, WebviewUrl::External(url))
                .on_navigation(|url| matches!(url.scheme(), "http" | "https"))
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny);
            #[cfg(not(target_os = "macos"))]
            let builder = builder.data_directory(
                app.path()
                    .app_data_dir()
                    .map_err(|_| "无法获取 Web 数据目录。")?
                    .join("web-data")
                    .join(&s.id),
            );
            #[cfg(target_os = "macos")]
            let builder = builder.data_store_identifier(
                *uuid::Uuid::parse_str(&s.id)
                    .map_err(|_| "服务标识无效。")?
                    .as_bytes(),
            );
            // Remote views have no matching capability and every custom command also
            // checks the caller label and local origin. No initialization IPC is added.
            let window = app.get_window("main").ok_or("主窗口不可用。")?;
            window
                .add_child(
                    builder,
                    position(app, b)?,
                    LogicalSize::new(b.width, b.height),
                )
                .map_err(|_| "无法创建服务 WebView。请检查系统 WebView 运行环境。")?;
            self.labels.insert(label.clone());
        }
        if let Some(w) = app.get_webview(&label) {
            w.set_position(position(app, b)?)
                .map_err(|_| "无法定位服务视图。")?;
            w.set_size(LogicalSize::new(b.width, b.height))
                .map_err(|_| "无法调整服务视图。")?;
            w.show().map_err(|_| "无法显示服务视图。")?;
        }
        self.active = Some(label);
        Ok(())
    }
    pub fn resize(&self, app: &tauri::AppHandle, b: &Bounds) -> AppResult<()> {
        b.validate()?;
        if let Some(w) = self.active.as_ref().and_then(|l| app.get_webview(l)) {
            w.set_position(position(app, b)?)
                .map_err(|_| "无法定位服务视图。")?;
            w.set_size(LogicalSize::new(b.width, b.height))
                .map_err(|_| "无法调整服务视图。")?;
        }
        Ok(())
    }
    pub fn close(&mut self, app: &tauri::AppHandle, id: &str) -> AppResult<()> {
        let label = format!("service-{id}");
        if let Some(w) = app.get_webview(&label) {
            w.close().map_err(|_| "无法关闭服务视图。")?;
        }
        self.labels.remove(&label);
        if self.active.as_ref() == Some(&label) {
            self.active = None;
        }
        Ok(())
    }
    pub fn close_all(&mut self, app: &tauri::AppHandle) -> AppResult<()> {
        for label in self.labels.clone() {
            if let Some(w) = app.get_webview(&label) {
                w.close().map_err(|_| "无法关闭服务视图。")?;
            }
        }
        self.labels.clear();
        self.active = None;
        Ok(())
    }
}
