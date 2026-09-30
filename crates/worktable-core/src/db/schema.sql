CREATE TABLE IF NOT EXISTS app_settings (
 id INTEGER PRIMARY KEY CHECK(id=1), lab_name TEXT NOT NULL,
 cluster_health_check_url TEXT NOT NULL, autostart INTEGER NOT NULL DEFAULT 0,
 start_in_tray INTEGER NOT NULL DEFAULT 0, vpn_autoconnect INTEGER NOT NULL DEFAULT 0,
 last_vpn_profile TEXT, update_endpoint TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS vpn_profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS services (id TEXT PRIMARY KEY, name TEXT NOT NULL, url TEXT NOT NULL, icon TEXT NOT NULL, requires_vpn INTEGER NOT NULL, favorite INTEGER NOT NULL, pinned_to_sidebar INTEGER NOT NULL, sort_order INTEGER NOT NULL, enabled INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS nodes (id TEXT PRIMARY KEY, display_name TEXT NOT NULL, ssh_host TEXT NOT NULL, enabled INTEGER NOT NULL, sort_order INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS todos (id TEXT PRIMARY KEY, title TEXT NOT NULL, completed INTEGER NOT NULL, due_date TEXT);
CREATE TABLE IF NOT EXISTS calendar_events (id TEXT PRIMARY KEY, title TEXT NOT NULL, starts_at TEXT NOT NULL, ends_at TEXT NOT NULL, notes TEXT NOT NULL, remind_at TEXT, notified_at TEXT);
CREATE TABLE IF NOT EXISTS sidebar_items (id TEXT PRIMARY KEY, sort_order INTEGER NOT NULL);
PRAGMA user_version=1;
