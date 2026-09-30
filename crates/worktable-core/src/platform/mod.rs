use crate::validation::{self, AppResult};
use serde::Serialize;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[derive(Serialize)]
pub struct Capability {
    pub available: bool,
    pub source: String,
    pub detail: String,
    pub platform: String,
}
pub fn find_executable(name: &str, extra: &[&str]) -> Option<PathBuf> {
    extra
        .iter()
        .map(PathBuf::from)
        .chain(std::env::var_os("PATH").into_iter().flat_map(|p| {
            std::env::split_paths(&p)
                .map(|dir| dir.join(name))
                .collect::<Vec<_>>()
        }))
        .find(|p| p.is_file())
}
pub fn vpn_privilege_guidance() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS：需有效的 OpenVPN 与 TUN/路由权限。当前尚未提供签名 privileged helper，普通用户启动可能失败；不会自动执行 sudo 或提权。"
    } else if cfg!(windows) {
        "Windows：需 OpenVPN、Wintun/TAP 驱动和受限权限服务。当前尚未提供驱动安装与特权服务；普通用户连接可能失败。"
    } else {
        "Linux：需 OpenVPN、/dev/net/tun 和受限 polkit/root helper。当前尚未提供 helper；普通用户连接可能失败。"
    }
}
fn vscode() -> AppResult<PathBuf> {
    #[cfg(target_os = "macos")]
    let extra = vec!["/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code"];
    #[cfg(not(target_os = "macos"))]
    let extra: Vec<&str> = Vec::new();
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let p = PathBuf::from(local).join("Programs/Microsoft VS Code/bin/code.cmd");
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    find_executable(if cfg!(windows) { "code.cmd" } else { "code" }, &extra)
        .ok_or_else(|| "未找到 VS Code。请先安装 VS Code，并将 code 命令加入 PATH。".into())
}
fn code_command() -> AppResult<Command> {
    let path = vscode()?;
    #[cfg(windows)]
    {
        // Invoke Code.exe's Node CLI directly, avoiding cmd.exe and .cmd argument parsing.
        let root = path
            .parent()
            .and_then(|p| p.parent())
            .ok_or("VS Code 安装路径无效。")?;
        let mut c = Command::new(root.join("Code.exe"));
        c.env("ELECTRON_RUN_AS_NODE", "1")
            .arg(root.join("resources/app/out/cli.js"));
        Ok(c)
    }
    #[cfg(not(windows))]
    {
        Ok(Command::new(path))
    }
}
pub fn open_vscode(host: &str) -> AppResult<()> {
    validation::ssh_host(host)?;
    let mut check = code_command()?;
    check
        .arg("--list-extensions")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = check.spawn().map_err(|_| "无法运行 VS Code 检测命令。")?;
    let stdout = child.stdout.take().ok_or("无法读取 VS Code 扩展列表。")?;
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        let _ = stdout.take(2 * 1024 * 1024).read_to_string(&mut text);
        text
    });
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err("VS Code 扩展检测失败，请检查本机安装。".into());
                }
                break;
            }
            Ok(None) => {}
            Err(_) => return Err("无法检测 VS Code 状态。".into()),
        }
        if Instant::now() > until {
            let _ = child.kill();
            let _ = child.wait();
            return Err("VS Code 扩展检测超时，请在本机确认 Remote - SSH 已安装。".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = reader.join().map_err(|_| "无法读取 VS Code 扩展信息。")?;
    if !output
        .lines()
        .any(|s| s.trim().eq_ignore_ascii_case("ms-vscode-remote.remote-ssh"))
    {
        return Err(
            "VS Code 尚未安装 Remote - SSH。请在扩展中安装 Microsoft Remote - SSH 后重试。".into(),
        );
    }
    let mut child = code_command()?
        .args(["--new-window", "--remote", &format!("ssh-remote+{host}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "无法打开 VS Code Remote SSH。")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
pub fn open_url(value: &str) -> AppResult<()> {
    let url = validation::web_url(value)?;
    #[cfg(target_os = "macos")]
    let mut c = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut c = Command::new("xdg-open");
    #[cfg(windows)]
    let mut c = {
        let mut c = Command::new("rundll32.exe");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    let mut child = c
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "无法打开系统浏览器。")?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
