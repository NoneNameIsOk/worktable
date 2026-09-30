use crate::{
    model::VpnProfile,
    validation::{self, AppResult},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub struct VpnProfileManager {
    root: PathBuf,
}
impl VpnProfileManager {
    pub fn new(data_dir: &Path) -> AppResult<Self> {
        let root = data_dir.join("vpn");
        fs::create_dir_all(&root).map_err(|_| "无法创建 VPN 私有目录。")?;
        private_permissions(&root, true)?;
        Ok(Self {
            root: root.canonicalize().map_err(|_| "无法访问 VPN 私有目录。")?,
        })
    }
    pub fn path(&self, id: &str) -> AppResult<PathBuf> {
        validation::id(id)?;
        let path = self.root.join(format!("{id}.ovpn"));
        if path.exists() {
            let canonical = path.canonicalize().map_err(|_| "无法访问 VPN 配置。")?;
            if canonical.parent() != Some(self.root.as_path())
                || fs::symlink_metadata(&path)
                    .map_err(|_| "无法访问 VPN 配置。")?
                    .file_type()
                    .is_symlink()
            {
                return Err("VPN 配置路径不安全。".into());
            }
        }
        Ok(path)
    }
    pub fn import(&self, source: &Path) -> AppResult<VpnProfile> {
        let source = source.canonicalize().map_err(|_| "无法读取所选文件。")?;
        if source
            .extension()
            .and_then(|s| s.to_str())
            .is_none_or(|s| !s.eq_ignore_ascii_case("ovpn"))
        {
            return Err("请选择 .ovpn 单文件配置。".into());
        }
        let meta = fs::metadata(&source).map_err(|_| "无法读取所选文件。")?;
        if !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
            return Err("配置必须是小于 2 MB 的普通 .ovpn 文件。".into());
        }
        let content =
            fs::read_to_string(&source).map_err(|_| "配置文件必须采用 UTF-8 文本格式。")?;
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("实验室 VPN");
        self.import_content(name, &content)
    }
    /// HTTP uploads never supply a filesystem path. Only the display name is retained.
    pub fn import_content(&self, name: &str, content: &str) -> AppResult<VpnProfile> {
        validation::title(name)?;
        if content.len() > 2 * 1024 * 1024 {
            return Err("配置必须小于 2 MB。".into());
        }
        preflight(content)?;
        let p = VpnProfile {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
        };
        secure_write(&self.path(&p.id)?, content.as_bytes())?;
        Ok(p)
    }
    pub fn remove(&self, id: &str) -> AppResult<()> {
        let p = self.path(id)?;
        match fs::remove_file(p) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("无法删除 VPN 私有配置。".into()),
        }
    }
    pub fn password_file(&self, secret: &str) -> AppResult<PathBuf> {
        let p = self
            .root
            .join(format!("management-{}.tmp", uuid::Uuid::new_v4()));
        secure_write(&p, secret.as_bytes())?;
        Ok(p)
    }
}
pub fn secure_write(path: &Path, content: &[u8]) -> AppResult<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path).map_err(|_| "无法创建私有文件。")?;
    if let Err(e) = private_permissions(path, false).and_then(|_| {
        f.write_all(content)
            .map_err(|_| "无法写入私有文件。".into())
    }) {
        let _ = fs::remove_file(path);
        return Err(e);
    }
    Ok(())
}
pub fn private_permissions(path: &Path, directory: bool) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )
        .map_err(|_| "无法限制私有文件权限。")?;
    }
    #[cfg(windows)]
    {
        let account = std::env::var("USERNAME").map_err(|_| "无法确定当前 Windows 用户。")?;
        let permission = if directory {
            format!("{account}:(OI)(CI)F")
        } else {
            format!("{account}:F")
        };
        let status = std::process::Command::new("icacls")
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &permission])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "无法设置 Windows 私有文件 ACL。")?;
        if !status.success() {
            return Err("无法限制 Windows 私有文件 ACL，已取消导入。".into());
        }
    }
    Ok(())
}
// A positive list deliberately rejects options which read/write files, load plugins,
// run hooks, include configs, daemonize, or replace our management channel.
pub fn preflight(content: &str) -> AppResult<()> {
    let allowed = [
        "client",
        "tls-client",
        "dev",
        "dev-type",
        "proto",
        "remote",
        "remote-random",
        "remote-random-hostname",
        "resolv-retry",
        "nobind",
        "persist-key",
        "persist-tun",
        "remote-cert-tls",
        "verify-x509-name",
        "cipher",
        "data-ciphers",
        "data-ciphers-fallback",
        "auth",
        "auth-nocache",
        "tls-version-min",
        "tls-version-max",
        "tls-cipher",
        "tls-ciphersuites",
        "key-direction",
        "verb",
        "mute",
        "explicit-exit-notify",
        "connect-retry",
        "connect-retry-max",
        "connect-timeout",
        "server-poll-timeout",
        "ping",
        "ping-restart",
        "keepalive",
        "float",
        "tun-mtu",
        "mssfix",
        "sndbuf",
        "rcvbuf",
        "pull",
        "pull-filter",
        "route",
        "route-ipv6",
        "route-metric",
        "route-nopull",
        "redirect-gateway",
        "dhcp-option",
        "block-outside-dns",
        "register-dns",
        "allow-compression",
        "compress",
        "comp-lzo",
        "fast-io",
        "reneg-sec",
        "auth-retry",
    ];
    let blocks = [
        "ca",
        "cert",
        "key",
        "tls-auth",
        "tls-crypt",
        "tls-crypt-v2",
        "peer-fingerprint",
    ];
    let mut block: Option<String> = None;
    let mut has_remote = false;
    let mut has_ca = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(b) = &block {
            if line == format!("</{b}>") {
                block = None;
            }
            continue;
        }
        if line.starts_with('<') && line.ends_with('>') {
            let b = &line[1..line.len() - 1];
            if !blocks.contains(&b) {
                return Err("配置包含不支持的内联区块。仅接受自包含的证书和密钥区块。".into());
            }
            has_ca |= b == "ca" || b == "peer-fingerprint";
            block = Some(b.into());
            continue;
        }
        if line.contains('\0') || line.ends_with('\\') {
            return Err("配置包含不支持的转义或多行 directive。".into());
        }
        let mut tokens = line.split_whitespace();
        let directive = tokens.next().unwrap_or("");
        if !allowed.contains(&directive) {
            return Err("配置包含不支持或有风险的 directive（外部文件、脚本、插件、日志或 management 设置等）。请由管理员提供 self-contained 配置；详情见 docs/VPN.md。".into());
        }
        if directive == "dev" && !matches!(tokens.next(), Some("tun" | "tap")) {
            return Err("第一版仅支持 dev tun 或 dev tap。".into());
        }
        if directive == "remote" {
            has_remote = true;
        }
    }
    if block.is_some() || !has_remote || !has_ca {
        return Err(
            "配置不完整：需要 remote、内联 CA（或 peer-fingerprint），且所有内联区块必须闭合。"
                .into(),
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    const CONFIG: &str =
        "client\ndev tun\nremote vpn.example.org 1194\n<ca>\nTEST CERTIFICATE\n</ca>\n";
    #[test]
    fn rejects_execution_and_external_files() {
        assert!(preflight(CONFIG).is_ok());
        for bad in [
            "up /tmp/run",
            "plugin /tmp/x",
            "management 0.0.0.0 10",
            "config /tmp/other",
            "ca ca.crt",
            "log /tmp/out",
            "script-security 2",
            "--up /tmp/a",
            "daemon",
        ] {
            assert!(preflight(&format!("{CONFIG}{bad}\n")).is_err());
        }
    }
    #[test]
    fn copies_private_and_rejects_traversal() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("test.ovpn");
        fs::write(&source, CONFIG).unwrap();
        let manager = VpnProfileManager::new(&root.path().join("private")).unwrap();
        assert!(manager.path("../../bad").is_err());
        let p = manager.import(&source).unwrap();
        fs::remove_file(source).unwrap();
        let copied = manager.path(&p.id).unwrap();
        assert!(copied.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(copied).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let manager = VpnProfileManager::new(root.path()).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        symlink("/etc/passwd", manager.path(&id).unwrap()).unwrap();
        assert!(manager.path(&id).is_err());
    }
}
