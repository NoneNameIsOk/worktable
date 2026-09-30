use crate::model::*;
use std::collections::HashSet;
pub type AppResult<T> = Result<T, String>;
pub fn title(value: &str) -> AppResult<()> {
    if value.trim().is_empty() || value.chars().count() > 200 {
        return Err("名称不能为空，且不能超过 200 字。".into());
    }
    Ok(())
}
pub fn id(value: &str) -> AppResult<()> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| "记录标识无效。".into())
}
pub fn web_url(value: &str) -> AppResult<url::Url> {
    let u = url::Url::parse(value).map_err(|_| "请输入完整的 http:// 或 https:// 地址。")?;
    if !matches!(u.scheme(), "http" | "https")
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err(
            "地址仅支持 HTTP/HTTPS，不能包含用户名、密码、查询参数或片段。请使用服务入口地址。"
                .into(),
        );
    }
    Ok(u)
}
pub fn ssh_host(host: &str) -> AppResult<()> {
    if host.is_empty()
        || host.len() > 253
        || !host.as_bytes()[0].is_ascii_alphanumeric()
        || !host
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        return Err("SSH Host 仅支持字母、数字、点、下划线和短横线，且以字母或数字开头。请填写已配置的 SSH 别名或主机名。".into());
    }
    Ok(())
}
pub fn service(s: &Service) -> AppResult<()> {
    id(&s.id)?;
    title(&s.name)?;
    web_url(&s.url)?;
    if s.icon.chars().count() > 16 {
        return Err("服务图标过长。".into());
    }
    Ok(())
}
pub fn node(n: &Node) -> AppResult<()> {
    id(&n.id)?;
    title(&n.display_name)?;
    ssh_host(&n.ssh_host)
}
pub fn settings(s: &Settings) -> AppResult<()> {
    title(&s.lab_name)?;
    web_url(&s.cluster_health_check_url)?;
    if !s.update_endpoint.is_empty() && web_url(&s.update_endpoint)?.scheme() != "https" {
        return Err("更新地址必须使用 HTTPS。".into());
    }
    Ok(())
}
pub fn event(e: &CalendarEvent) -> AppResult<()> {
    id(&e.id)?;
    title(&e.title)?;
    let start =
        chrono::DateTime::parse_from_rfc3339(&e.starts_at).map_err(|_| "开始时间格式无效。")?;
    let end = chrono::DateTime::parse_from_rfc3339(&e.ends_at).map_err(|_| "结束时间格式无效。")?;
    if end < start {
        return Err("结束时间不能早于开始时间。".into());
    }
    if e.notes.len() > 20000 {
        return Err("日程备注过长。".into());
    }
    if let Some(r) = &e.remind_at {
        let r = chrono::DateTime::parse_from_rfc3339(r).map_err(|_| "提醒时间格式无效。")?;
        if r > start {
            return Err("提醒时间不能晚于开始时间。".into());
        }
    }
    Ok(())
}
pub fn lab_config(c: &LabConfig) -> AppResult<()> {
    if c.schema_version != 1 {
        return Err("不支持此配置包版本，当前仅支持 schemaVersion = 1。".into());
    }
    title(&c.lab_name)?;
    web_url(&c.cluster_health_check_url)?;
    if c.services.len() > 500 || c.nodes.len() > 500 {
        return Err("配置包最多包含 500 个服务和 500 个节点。".into());
    }
    let mut ids = HashSet::new();
    for s in &c.services {
        service(s)?;
        if !ids.insert(&s.id) {
            return Err("配置包包含重复标识。".into());
        }
    }
    for n in &c.nodes {
        node(n)?;
        if !ids.insert(&n.id) {
            return Err("配置包包含重复标识。".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_urls() {
        for u in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://u:p@example.com",
            "https://example.com/?token=secret",
        ] {
            assert!(web_url(u).is_err());
        }
        assert!(web_url("http://192.168.100.1:8080/").is_ok());
    }
    #[test]
    fn rejects_shell_hosts() {
        for h in [
            "-oProxyCommand=evil",
            "node;rm",
            "$(whoami)",
            "user@host",
            "a\nb",
        ] {
            assert!(ssh_host(h).is_err());
        }
        assert!(ssh_host("vipsl10").is_ok());
    }
}
