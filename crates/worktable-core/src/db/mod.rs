use crate::{
    model::*,
    validation::{self, AppResult},
};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database(pub Connection);
fn db_err(_: rusqlite::Error) -> String {
    "本地数据库操作失败，请检查磁盘空间和目录权限。".into()
}
impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::from_connection(Connection::open(path).map_err(db_err)?)
    }
    fn from_connection(mut c: Connection) -> AppResult<Self> {
        c.pragma_update(None, "journal_mode", "WAL")
            .map_err(db_err)?;
        c.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_err)?;
        let version: u32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db_err)?;
        if version > 1 {
            return Err("数据库由更新版本创建，请升级 Worktable。".into());
        }
        if version == 0 {
            let tx = c.transaction().map_err(db_err)?;
            tx.execute_batch(include_str!("schema.sql"))
                .map_err(db_err)?;
            let s = Settings::default();
            tx.execute(
                "INSERT INTO app_settings(id,lab_name,cluster_health_check_url) VALUES(1,?1,?2)",
                params![s.lab_name, s.cluster_health_check_url],
            )
            .map_err(db_err)?;
            for i in 1..=15 {
                tx.execute(
                    "INSERT INTO nodes VALUES(?1,?2,?2,1,?3)",
                    params![uuid::Uuid::new_v4().to_string(), format!("vipsl{i}"), i],
                )
                .map_err(db_err)?;
            }
            for (i, name) in MODULE_IDS.iter().enumerate() {
                tx.execute(
                    "INSERT INTO sidebar_items VALUES(?1,?2)",
                    params![name, i as i64],
                )
                .map_err(db_err)?;
            }
            tx.commit().map_err(db_err)?;
        }
        Ok(Self(c))
    }
    pub fn settings(&self) -> AppResult<Settings> {
        self.0.query_row("SELECT lab_name,cluster_health_check_url,autostart,start_in_tray,vpn_autoconnect,last_vpn_profile,update_endpoint FROM app_settings WHERE id=1", [], |r| Ok(Settings { lab_name:r.get(0)?,cluster_health_check_url:r.get(1)?,autostart:r.get(2)?,start_in_tray:r.get(3)?,vpn_autoconnect:r.get(4)?,last_vpn_profile:r.get(5)?,update_endpoint:r.get(6)? })).map_err(db_err)
    }
    pub fn snapshot(&self) -> AppResult<Snapshot> {
        macro_rules! rows {
            ($sql:expr, $map:expr) => {{
                let mut stmt = self.0.prepare($sql).map_err(db_err)?;
                let values = stmt
                    .query_map([], $map)
                    .map_err(db_err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(db_err)?;
                values
            }};
        }
        Ok(Snapshot {
            settings: self.settings()?,
            services: rows!("SELECT * FROM services ORDER BY sort_order,name", |r| Ok(
                Service {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    url: r.get(2)?,
                    icon: r.get(3)?,
                    requires_vpn: r.get(4)?,
                    favorite: r.get(5)?,
                    pinned_to_sidebar: r.get(6)?,
                    sort_order: r.get(7)?,
                    enabled: r.get(8)?
                }
            )),
            nodes: rows!(
                "SELECT * FROM nodes ORDER BY sort_order,display_name",
                |r| Ok(Node {
                    id: r.get(0)?,
                    display_name: r.get(1)?,
                    ssh_host: r.get(2)?,
                    enabled: r.get(3)?,
                    sort_order: r.get(4)?
                })
            ),
            todos: rows!("SELECT * FROM todos ORDER BY completed,rowid DESC", |r| Ok(
                Todo {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    completed: r.get(2)?,
                    due_date: r.get(3)?
                }
            )),
            calendar_events: rows!("SELECT * FROM calendar_events ORDER BY starts_at", |r| Ok(
                CalendarEvent {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    starts_at: r.get(2)?,
                    ends_at: r.get(3)?,
                    notes: r.get(4)?,
                    remind_at: r.get(5)?,
                    notified_at: r.get(6)?
                }
            )),
            sidebar_items: rows!("SELECT * FROM sidebar_items ORDER BY sort_order", |r| Ok(
                SidebarItem {
                    id: r.get(0)?,
                    sort_order: r.get(1)?
                }
            )),
            vpn_profiles: rows!("SELECT * FROM vpn_profiles ORDER BY rowid", |r| Ok(
                VpnProfile {
                    id: r.get(0)?,
                    name: r.get(1)?
                }
            )),
        })
    }
    pub fn mutate(&mut self, m: Mutation) -> AppResult<()> {
        let tx = self.0.transaction().map_err(db_err)?;
        match m {
            Mutation::SaveTodo(t) => {
                validation::id(&t.id)?;
                validation::title(&t.title)?;
                if let Some(d) = &t.due_date {
                    chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                        .map_err(|_| "截止日期无效。")?;
                }
                tx.execute("INSERT INTO todos VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,completed=excluded.completed,due_date=excluded.due_date",params![t.id,t.title,t.completed,t.due_date]).map_err(db_err)?;
            }
            Mutation::SaveEvent(e) => {
                validation::event(&e)?;
                tx.execute("INSERT INTO calendar_events VALUES(?1,?2,?3,?4,?5,?6,NULL) ON CONFLICT(id) DO UPDATE SET title=excluded.title,starts_at=excluded.starts_at,ends_at=excluded.ends_at,notes=excluded.notes,remind_at=excluded.remind_at,notified_at=CASE WHEN calendar_events.remind_at IS excluded.remind_at AND calendar_events.starts_at=excluded.starts_at THEN calendar_events.notified_at ELSE NULL END",params![e.id,e.title,e.starts_at,e.ends_at,e.notes,e.remind_at]).map_err(db_err)?;
            }
            Mutation::SaveService(s) => {
                validation::service(&s)?;
                save_service(&tx, &s)?;
                sync_pin(&tx, &s)?;
            }
            Mutation::SaveNode(n) => {
                validation::node(&n)?;
                save_node(&tx, &n)?;
            }
            Mutation::DeleteTodo(id) => {
                tx.execute("DELETE FROM todos WHERE id=?1", [id])
                    .map_err(db_err)?;
            }
            Mutation::DeleteEvent(id) => {
                tx.execute("DELETE FROM calendar_events WHERE id=?1", [id])
                    .map_err(db_err)?;
            }
            Mutation::DeleteNode(id) => {
                tx.execute("DELETE FROM nodes WHERE id=?1", [id])
                    .map_err(db_err)?;
            }
            Mutation::DeleteService(id) => {
                tx.execute(
                    "DELETE FROM sidebar_items WHERE id=?1",
                    [format!("service:{id}")],
                )
                .map_err(db_err)?;
                tx.execute("DELETE FROM services WHERE id=?1", [id])
                    .map_err(db_err)?;
            }
            Mutation::SaveSettings(s) => {
                validation::settings(&s)?;
                tx.execute("UPDATE app_settings SET lab_name=?1,cluster_health_check_url=?2,autostart=?3,start_in_tray=?4,vpn_autoconnect=?5,update_endpoint=?6 WHERE id=1",params![s.lab_name,s.cluster_health_check_url,s.autostart,s.start_in_tray,s.vpn_autoconnect,s.update_endpoint]).map_err(db_err)?;
            }
            Mutation::RenameProfile(p) => {
                validation::title(&p.name)?;
                tx.execute(
                    "UPDATE vpn_profiles SET name=?1 WHERE id=?2",
                    params![p.name, p.id],
                )
                .map_err(db_err)?;
            }
            Mutation::ReorderSidebar(ids) => {
                let current: Vec<String> = tx
                    .prepare("SELECT id FROM sidebar_items")
                    .map_err(db_err)?
                    .query_map([], |r| r.get(0))
                    .map_err(db_err)?
                    .collect::<Result<_, _>>()
                    .map_err(db_err)?;
                let set: std::collections::HashSet<_> = ids.iter().collect();
                if ids.len() != current.len()
                    || set.len() != ids.len()
                    || !current.iter().all(|id| set.contains(id))
                {
                    return Err("侧栏顺序不完整，请刷新后重试。".into());
                }
                for (i, id) in ids.iter().enumerate() {
                    tx.execute(
                        "UPDATE sidebar_items SET sort_order=?1 WHERE id=?2",
                        params![i as i64, id],
                    )
                    .map_err(db_err)?;
                }
            }
        }
        tx.commit().map_err(db_err)
    }
    pub fn export_lab(&self) -> AppResult<LabConfig> {
        let s = self.snapshot()?;
        Ok(LabConfig {
            schema_version: 1,
            lab_name: s.settings.lab_name,
            cluster_health_check_url: s.settings.cluster_health_check_url,
            services: s.services,
            nodes: s.nodes,
            default_settings: PublicDefaults {
                start_in_tray: s.settings.start_in_tray,
            },
        })
    }
    pub fn import_lab(&mut self, c: LabConfig) -> AppResult<()> {
        validation::lab_config(&c)?;
        let tx = self.0.transaction().map_err(db_err)?;
        tx.execute("DELETE FROM services", []).map_err(db_err)?;
        tx.execute("DELETE FROM nodes", []).map_err(db_err)?;
        tx.execute("DELETE FROM sidebar_items WHERE id LIKE 'service:%'", [])
            .map_err(db_err)?;
        for s in c.services {
            save_service(&tx, &s)?;
            sync_pin(&tx, &s)?;
        }
        for n in c.nodes {
            save_node(&tx, &n)?;
        }
        tx.execute("UPDATE app_settings SET lab_name=?1,cluster_health_check_url=?2,start_in_tray=?3 WHERE id=1",params![c.lab_name,c.cluster_health_check_url,c.default_settings.start_in_tray]).map_err(db_err)?;
        tx.commit().map_err(db_err)
    }
    pub fn add_profile(&self, p: &VpnProfile) -> AppResult<()> {
        self.0
            .execute(
                "INSERT INTO vpn_profiles VALUES(?1,?2)",
                params![p.id, p.name],
            )
            .map(|_| ())
            .map_err(db_err)
    }
    pub fn delete_profile(&mut self, id: &str) -> AppResult<()> {
        let tx = self.0.transaction().map_err(db_err)?;
        tx.execute("DELETE FROM vpn_profiles WHERE id=?1", [id])
            .map_err(db_err)?;
        tx.execute(
            "UPDATE app_settings SET last_vpn_profile=NULL WHERE last_vpn_profile=?1",
            [id],
        )
        .map_err(db_err)?;
        tx.commit().map_err(db_err)
    }
    pub fn set_last_profile(&self, id: &str) -> AppResult<()> {
        self.0
            .execute("UPDATE app_settings SET last_vpn_profile=?1", [id])
            .map(|_| ())
            .map_err(db_err)
    }
    pub fn due_reminders(&self) -> AppResult<Vec<(String, String)>> {
        let mut s=self.0.prepare("SELECT id,title FROM calendar_events WHERE notified_at IS NULL AND remind_at IS NOT NULL AND julianday(remind_at)<=julianday('now') AND julianday(ends_at)>=julianday('now')").map_err(db_err)?;
        let v = s
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(db_err)?
            .collect::<Result<_, _>>()
            .map_err(db_err)?;
        Ok(v)
    }
    /// Atomically claim due reminders so multiple browser tabs cannot display duplicates.
    pub fn claim_reminders(&mut self) -> AppResult<Vec<(String, String)>> {
        let tx = self
            .0
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_err)?;
        let events: Vec<(String, String)> = {
            let mut q = tx.prepare("SELECT id,title FROM calendar_events WHERE notified_at IS NULL AND remind_at IS NOT NULL AND julianday(remind_at)<=julianday('now') AND julianday(ends_at)>=julianday('now')").map_err(db_err)?;
            let events = q
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(db_err)?
                .collect::<Result<_, _>>()
                .map_err(db_err)?;
            events
        };
        for (id, _) in &events {
            tx.execute(
                "UPDATE calendar_events SET notified_at=?1 WHERE id=?2",
                params![chrono::Utc::now().to_rfc3339(), id],
            )
            .map_err(db_err)?;
        }
        tx.commit().map_err(db_err)?;
        Ok(events)
    }
    pub fn mark_notified(&self, id: &str) -> AppResult<()> {
        self.0
            .execute(
                "UPDATE calendar_events SET notified_at=?1 WHERE id=?2",
                params![chrono::Utc::now().to_rfc3339(), id],
            )
            .map(|_| ())
            .map_err(db_err)
    }
}
fn save_service(c: &Connection, s: &Service) -> AppResult<()> {
    c.execute("INSERT INTO services VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET name=excluded.name,url=excluded.url,icon=excluded.icon,requires_vpn=excluded.requires_vpn,favorite=excluded.favorite,pinned_to_sidebar=excluded.pinned_to_sidebar,sort_order=excluded.sort_order,enabled=excluded.enabled",params![s.id,s.name,s.url,s.icon,s.requires_vpn,s.favorite,s.pinned_to_sidebar,s.sort_order,s.enabled]).map(|_|()).map_err(db_err)
}
fn save_node(c: &Connection, n: &Node) -> AppResult<()> {
    c.execute("INSERT INTO nodes VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,ssh_host=excluded.ssh_host,enabled=excluded.enabled,sort_order=excluded.sort_order",params![n.id,n.display_name,n.ssh_host,n.enabled,n.sort_order]).map(|_|()).map_err(db_err)
}
fn sync_pin(c: &Connection, s: &Service) -> AppResult<()> {
    let id = format!("service:{}", s.id);
    if s.pinned_to_sidebar {
        c.execute("INSERT OR IGNORE INTO sidebar_items SELECT ?1,COALESCE(MAX(sort_order),-1)+1 FROM sidebar_items",[id]).map_err(db_err)?;
    } else {
        c.execute("DELETE FROM sidebar_items WHERE id=?1", [id])
            .map_err(db_err)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Database {
        Database::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }
    #[test]
    fn todo_crud_persists() {
        let mut d = db();
        let mut t = Todo {
            id: uuid::Uuid::new_v4().to_string(),
            title: "写论文".into(),
            completed: false,
            due_date: None,
        };
        d.mutate(Mutation::SaveTodo(t.clone())).unwrap();
        t.completed = true;
        d.mutate(Mutation::SaveTodo(t.clone())).unwrap();
        assert!(d.snapshot().unwrap().todos[0].completed);
        d.mutate(Mutation::DeleteTodo(t.id)).unwrap();
        assert!(d.snapshot().unwrap().todos.is_empty());
    }
    #[test]
    fn import_version_and_sensitive_fields() {
        let mut d = db();
        let c = d.export_lab().unwrap();
        let json = serde_json::to_string(&c).unwrap();
        for key in [
            "vpnProfiles",
            "lastVpnProfile",
            "todos",
            "calendarEvents",
            "updateEndpoint",
            "password",
            "cookie",
        ] {
            assert!(!json.contains(key));
        }
        let mut wrong = c.clone();
        wrong.schema_version = 2;
        assert!(d.import_lab(wrong).is_err());
        assert_eq!(d.snapshot().unwrap().nodes.len(), 15);
        let mut value = serde_json::to_value(c).unwrap();
        value["password"] = serde_json::json!("secret");
        assert!(serde_json::from_value::<LabConfig>(value).is_err());
    }
    #[test]
    fn sidebar_cannot_remove_builtins() {
        let mut d = db();
        assert!(d
            .mutate(Mutation::ReorderSidebar(vec!["home".into()]))
            .is_err());
        let ids = MODULE_IDS.iter().rev().map(|s| s.to_string()).collect();
        d.mutate(Mutation::ReorderSidebar(ids)).unwrap();
        assert_eq!(d.snapshot().unwrap().sidebar_items[0].id, "settings");
    }
    #[test]
    fn reminder_deduplicates_and_reschedules() {
        let mut d = db();
        let now = chrono::Utc::now();
        let mut e = CalendarEvent {
            id: uuid::Uuid::new_v4().to_string(),
            title: "组会".into(),
            starts_at: now.to_rfc3339(),
            ends_at: (now + chrono::Duration::hours(1)).to_rfc3339(),
            notes: String::new(),
            remind_at: Some((now - chrono::Duration::minutes(1)).to_rfc3339()),
            notified_at: None,
        };
        d.mutate(Mutation::SaveEvent(e.clone())).unwrap();
        assert_eq!(d.due_reminders().unwrap().len(), 1);
        d.mark_notified(&e.id).unwrap();
        e.notes = "新备注".into();
        d.mutate(Mutation::SaveEvent(e.clone())).unwrap();
        assert!(d.due_reminders().unwrap().is_empty());
        e.remind_at = Some((now - chrono::Duration::minutes(2)).to_rfc3339());
        d.mutate(Mutation::SaveEvent(e)).unwrap();
        assert_eq!(d.due_reminders().unwrap().len(), 1);
    }
    #[test]
    fn web_reminders_are_claimed_once_across_connections() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("test.sqlite3");
        let mut first = Database::open(&path).unwrap();
        let mut second = Database::open(&path).unwrap();
        let now = chrono::Utc::now();
        first
            .mutate(Mutation::SaveEvent(CalendarEvent {
                id: uuid::Uuid::new_v4().to_string(),
                title: "网页提醒".into(),
                starts_at: now.to_rfc3339(),
                ends_at: (now + chrono::Duration::hours(1)).to_rfc3339(),
                notes: String::new(),
                remind_at: Some((now - chrono::Duration::minutes(1)).to_rfc3339()),
                notified_at: None,
            }))
            .unwrap();
        assert_eq!(first.claim_reminders().unwrap().len(), 1);
        assert!(second.claim_reminders().unwrap().is_empty());
    }
    #[test]
    fn invalid_import_is_atomic() {
        let mut d = db();
        let mut c = d.export_lab().unwrap();
        c.nodes[0].ssh_host = "a; bad".into();
        assert!(d.import_lab(c).is_err());
        assert_eq!(d.snapshot().unwrap().nodes[0].ssh_host, "vipsl1");
    }
}
