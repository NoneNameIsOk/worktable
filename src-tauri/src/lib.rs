mod commands;
use worktable_core::db;
use worktable_core::model;
use worktable_core::platform;
use worktable_core::updater;
use worktable_core::validation;
use worktable_core::vpn;
mod web_services;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;
use validation::AppResult;
use vpn::VpnManager;

pub struct AppState {
    db: Mutex<db::Database>,
    profiles: vpn::profile::VpnProfileManager,
    vpn: VpnManager,
    backend: vpn::BundledOpenVpnBackend,
    services: Mutex<web_services::ServiceViews>,
    exiting: AtomicBool,
}
impl AppState {
    fn db(&self) -> AppResult<std::sync::MutexGuard<'_, db::Database>> {
        self.db.lock().map_err(|_| "本地数据暂时不可用。".into())
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(MacosLauncher::LaunchAgent)
                .args(["--autostart"])
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::mutate,
            commands::vpn_status,
            commands::vpn_capability,
            commands::vpn_connect,
            commands::vpn_disconnect,
            commands::import_vpn_profile,
            commands::delete_vpn_profile,
            commands::import_lab_config,
            commands::export_lab_config,
            commands::show_service,
            commands::hide_services,
            commands::resize_service,
            commands::reset_service,
            commands::open_node,
            commands::probe_node,
            commands::cluster_health,
            commands::request_notifications,
            commands::check_update,
            commands::open_download
        ])
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            vpn::profile::private_permissions(&data, true)?;
            let database = db::Database::open(&data.join("worktable.sqlite3"))?;
            let settings = database.settings()?;
            app.manage(AppState {
                db: Mutex::new(database),
                profiles: vpn::profile::VpnProfileManager::new(&data)?,
                vpn: VpnManager::default(),
                backend: vpn::BundledOpenVpnBackend {
                    resource_dir: app.path().resource_dir()?,
                },
                services: Mutex::new(web_services::ServiceViews::default()),
                exiting: AtomicBool::new(false),
            });
            let open = MenuItem::with_id(app, "open", "打开 Worktable", true, None::<&str>)?;
            let status = MenuItem::with_id(app, "status", "VPN：未连接", false, None::<&str>)?;
            let connect =
                MenuItem::with_id(app, "connect", "连接上次使用的 VPN", true, None::<&str>)?;
            let disconnect = MenuItem::with_id(app, "disconnect", "断开 VPN", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 Worktable", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &status, &connect, &disconnect, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("HYKSJ Worktable")
                .menu(&menu)
                .show_menu_on_left_click(true);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "open" => {
                    if let Some(w) = app.get_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
                "disconnect" => {
                    let _ = app.state::<AppState>().vpn.disconnect();
                }
                "connect" => {
                    let state = app.state::<AppState>();
                    let id = state
                        .db()
                        .and_then(|d| d.settings())
                        .ok()
                        .and_then(|s| s.last_vpn_profile);
                    if let Some(id) = id {
                        if let Err(e) = commands::connect_profile(&state, &id) {
                            let _ = app
                                .notification()
                                .builder()
                                .title("VPN 连接未完成")
                                .body(e)
                                .show();
                        }
                    } else {
                        let _ = app
                            .notification()
                            .builder()
                            .title("请选择 VPN 配置")
                            .body("打开 Worktable，导入并连接一个 .ovpn 配置后即可使用托盘连接。")
                            .show();
                    }
                }
                "quit" => {
                    let app = app.clone();
                    app.state::<AppState>()
                        .exiting
                        .store(true, Ordering::SeqCst);
                    std::thread::spawn(move || {
                        app.state::<AppState>().vpn.shutdown();
                        app.exit(0);
                    });
                }
                _ => {}
            })
            .build(app)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut ticks = 0;
                loop {
                    let state = handle.state::<AppState>();
                    if state.exiting.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(vpn) = state.vpn.status() {
                        let name = match vpn.state {
                            vpn::VpnState::Disconnected => "未连接",
                            vpn::VpnState::Connecting => "连接中",
                            vpn::VpnState::Connected => "已连接",
                            vpn::VpnState::Reconnecting => "重连中",
                            vpn::VpnState::Disconnecting => "断开中",
                            vpn::VpnState::Failed => "连接失败",
                        };
                        let _ = status.set_text(format!("VPN：{name}"));
                    }
                    if ticks % 15 == 0 {
                        if let Ok(db) = state.db() {
                            if let Ok(events) = db.due_reminders() {
                                for (id, title) in events {
                                    if handle
                                        .notification()
                                        .builder()
                                        .title("Worktable 日程提醒")
                                        .body(&title)
                                        .show()
                                        .is_ok()
                                    {
                                        let _ = db.mark_notified(&id);
                                    }
                                }
                            }
                        }
                    }
                    ticks += 1;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
            if settings.start_in_tray {
                if let Some(w) = app.get_window("main") {
                    w.hide()?;
                }
            }
            if settings.vpn_autoconnect {
                if let Some(id) = settings.last_vpn_profile {
                    let state = app.state::<AppState>();
                    if let Err(e) = commands::connect_profile(&state, &id) {
                        let _ = app
                            .notification()
                            .builder()
                            .title("VPN 自动连接未完成")
                            .body(e)
                            .show();
                    }
                }
            }
            Ok(())
        })
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                if !w.state::<AppState>().exiting.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Worktable 初始化失败，请检查应用数据目录权限")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !app.state::<AppState>().exiting.swap(true, Ordering::SeqCst) {
                    api.prevent_exit();
                    let app = app.clone();
                    std::thread::spawn(move || {
                        app.state::<AppState>().vpn.shutdown();
                        app.exit(0);
                    });
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                if let Some(w) = app.get_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        });
}
