use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub lab_name: String,
    pub cluster_health_check_url: String,
    pub autostart: bool,
    pub start_in_tray: bool,
    pub vpn_autoconnect: bool,
    pub last_vpn_profile: Option<String>,
    pub update_endpoint: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            lab_name: "HYKSJ 实验室".into(),
            cluster_health_check_url: "http://192.168.100.1:8080/".into(),
            autostart: false,
            start_in_tray: false,
            vpn_autoconnect: false,
            last_vpn_profile: None,
            update_endpoint: String::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon: String,
    pub requires_vpn: bool,
    pub favorite: bool,
    pub pinned_to_sidebar: bool,
    pub sort_order: i64,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub display_name: String,
    pub ssh_host: String,
    pub enabled: bool,
    pub sort_order: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub starts_at: String,
    pub ends_at: String,
    pub notes: String,
    pub remind_at: Option<String>,
    pub notified_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SidebarItem {
    pub id: String,
    pub sort_order: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VpnProfile {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub services: Vec<Service>,
    pub nodes: Vec<Node>,
    pub todos: Vec<Todo>,
    pub calendar_events: Vec<CalendarEvent>,
    pub sidebar_items: Vec<SidebarItem>,
    pub vpn_profiles: Vec<VpnProfile>,
}
#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum Mutation {
    SaveTodo(Todo),
    DeleteTodo(String),
    SaveEvent(CalendarEvent),
    DeleteEvent(String),
    SaveService(Service),
    DeleteService(String),
    SaveNode(Node),
    DeleteNode(String),
    ReorderSidebar(Vec<String>),
    SaveSettings(Settings),
    RenameProfile(VpnProfile),
}
// Public configuration intentionally cannot represent local credentials or VPN data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicDefaults {
    pub start_in_tray: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LabConfig {
    pub schema_version: u32,
    pub lab_name: String,
    pub cluster_health_check_url: String,
    pub services: Vec<Service>,
    pub nodes: Vec<Node>,
    pub default_settings: PublicDefaults,
}
pub const MODULE_IDS: [&str; 7] = [
    "home",
    "services",
    "nodes",
    "experiments",
    "calendar",
    "plugins",
    "settings",
];
