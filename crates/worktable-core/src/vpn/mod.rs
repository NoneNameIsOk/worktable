pub mod profile;
use crate::{platform, validation::AppResult};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum VpnState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Disconnecting,
    Failed,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VpnStatus {
    pub state: VpnState,
    pub profile_id: Option<String>,
    pub message: String,
}
impl Default for VpnStatus {
    fn default() -> Self {
        Self {
            state: VpnState::Disconnected,
            profile_id: None,
            message: "VPN 未连接，本地功能可正常使用。".into(),
        }
    }
}
pub trait VpnBackend {
    fn capability(&self) -> platform::Capability;
    fn executable(&self) -> AppResult<PathBuf>;
}
pub struct BundledOpenVpnBackend {
    pub resource_dir: PathBuf,
}
impl VpnBackend for BundledOpenVpnBackend {
    fn executable(&self) -> AppResult<PathBuf> {
        let binary = if cfg!(windows) {
            "openvpn.exe"
        } else {
            "openvpn"
        };
        let bundled = self
            .resource_dir
            .join("binaries")
            .join(std::env::consts::OS)
            .join(std::env::consts::ARCH)
            .join(binary);
        if bundled.is_file() {
            return Ok(bundled);
        }
        if cfg!(debug_assertions) {
            if let Some(p) = platform::find_executable(
                binary,
                &[
                    "/opt/homebrew/sbin/openvpn",
                    "/usr/local/sbin/openvpn",
                    "/usr/sbin/openvpn",
                ],
            ) {
                return Ok(p);
            }
        }
        Err("尚未安装可用的 OpenVPN backend。开发构建可使用系统 OpenVPN；正式发布需提供随包二进制与平台权限组件，详见 docs/VPN.md。".into())
    }
    fn capability(&self) -> platform::Capability {
        match self.executable() {
            Ok(p) => platform::Capability {
                available: true,
                source: if p.starts_with(&self.resource_dir) {
                    "bundled"
                } else {
                    "system (开发模式)"
                }
                .into(),
                platform: std::env::consts::OS.into(),
                detail: platform::vpn_privilege_guidance().into(),
            },
            Err(e) => platform::Capability {
                available: false,
                source: "none".into(),
                platform: std::env::consts::OS.into(),
                detail: e,
            },
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
impl Credentials {
    pub fn validate(&self) -> AppResult<()> {
        if [&self.username, &self.password]
            .iter()
            .any(|v| v.is_empty() || v.len() > 1024 || v.chars().any(char::is_control))
        {
            return Err("请输入有效的 VPN 用户名和密码（不能包含换行或控制字符）。".into());
        }
        Ok(())
    }
    fn commands(&self) -> AppResult<String> {
        self.validate()?;
        let quote = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        Ok(format!(
            "username \"Auth\" \"{}\"\npassword \"Auth\" \"{}\"\n",
            quote(&self.username),
            quote(&self.password)
        ))
    }
}
pub struct VpnManager {
    status: Arc<Mutex<VpnStatus>>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}
impl Default for VpnManager {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(VpnStatus::default())),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }
}
impl VpnManager {
    pub fn status(&self) -> AppResult<VpnStatus> {
        self.status
            .lock()
            .map(|s| s.clone())
            .map_err(|_| "无法读取 VPN 状态。".into())
    }
    pub fn connect(
        &self,
        backend: &dyn VpnBackend,
        profile_id: String,
        profile: PathBuf,
        password_path: PathBuf,
        password: String,
        credentials: Option<Credentials>,
    ) -> AppResult<()> {
        let result = (|| {
            if let Some(c) = &credentials {
                c.validate()?;
            }
            let mut worker = self.worker.lock().map_err(|_| "VPN 后台任务繁忙。")?;
            if worker.as_ref().is_some_and(|w| !w.is_finished()) {
                return Err("已有 VPN 任务运行中，请先断开。".into());
            }
            if let Some(old) = worker.take() {
                let _ = old.join();
            }
            let binary = backend.executable()?;
            let listener = TcpListener::bind(("127.0.0.1", 0))
                .map_err(|_| "无法分配本地 VPN management 端口。")?;
            let port = listener
                .local_addr()
                .map_err(|_| "无法读取本地端口。")?
                .port();
            drop(listener);
            self.stop.store(false, Ordering::SeqCst);
            let child = Command::new(binary)
                .arg("--config")
                .arg(profile)
                .args(["--management", "127.0.0.1", &port.to_string()])
                .arg(&password_path)
                .args([
                    "--management-hold",
                    "--management-query-passwords",
                    "--auth-retry",
                    "none",
                    "--script-security",
                    "1",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| "无法启动 OpenVPN，请检查二进制权限与依赖。")?;
            *self.status.lock().map_err(|_| "VPN 状态不可用。")? = VpnStatus {
                state: VpnState::Connecting,
                profile_id: Some(profile_id),
                message: "正在连接 OpenVPN management interface…".into(),
            };
            let state = Arc::clone(&self.status);
            let stop = Arc::clone(&self.stop);
            let secret_path = password_path.clone();
            *worker = Some(thread::spawn(move || {
                run_management(child, port, &password, credentials, &state, &stop);
                let _ = std::fs::remove_file(secret_path);
            }));
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(password_path);
        }
        result
    }
    pub fn disconnect(&self) -> AppResult<()> {
        self.stop.store(true, Ordering::SeqCst);
        let mut s = self.status.lock().map_err(|_| "VPN 状态不可用。")?;
        if matches!(
            s.state,
            VpnState::Connecting | VpnState::Connected | VpnState::Reconnecting
        ) {
            s.state = VpnState::Disconnecting;
            s.message = "正在断开 VPN…".into();
        }
        Ok(())
    }
    pub fn shutdown(&self) {
        let _ = self.disconnect();
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(w) = worker.take() {
                let _ = w.join();
            }
        }
    }
}
fn set(state: &Mutex<VpnStatus>, s: VpnState, message: &str) {
    if let Ok(mut status) = state.lock() {
        status.state = s;
        status.message = message.into();
    }
}
fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
fn fail(child: &mut Child, state: &Mutex<VpnStatus>, message: &str) {
    terminate(child);
    set(state, VpnState::Failed, message);
}
pub fn management_state(line: &str) -> Option<VpnState> {
    let payload = line.strip_prefix(">STATE:")?;
    let s = payload.split(',').nth(1)?;
    match s {
        "CONNECTED" => Some(VpnState::Connected),
        "RECONNECTING" => Some(VpnState::Reconnecting),
        "EXITING" => Some(VpnState::Disconnected),
        "RESOLVE" | "TCP_CONNECT" | "WAIT" | "AUTH" | "GET_CONFIG" | "ASSIGN_IP" | "ADD_ROUTES" => {
            Some(VpnState::Connecting)
        }
        _ => None,
    }
}
fn run_management(
    mut child: Child,
    port: u16,
    password: &str,
    credentials: Option<Credentials>,
    state: &Mutex<VpnStatus>,
    stop: &AtomicBool,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut stream = loop {
        if stop.load(Ordering::SeqCst) {
            terminate(&mut child);
            set(state, VpnState::Disconnected, "VPN 已断开。");
            return;
        }
        if matches!(child.try_wait(), Ok(Some(_))) {
            set(state, VpnState::Failed, platform::vpn_privilege_guidance());
            return;
        }
        if let Ok(s) = TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(300),
        ) {
            break s;
        }
        if Instant::now() > deadline {
            fail(
                &mut child,
                state,
                "无法连接 OpenVPN management interface，请检查 OpenVPN 版本、配置和系统权限。",
            );
            return;
        }
        thread::sleep(Duration::from_millis(100));
    };
    if stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .is_err()
        || stream
            .set_write_timeout(Some(Duration::from_secs(1)))
            .is_err()
    {
        fail(&mut child, state, "无法初始化 VPN management 通道。");
        return;
    }
    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => {
            fail(&mut child, state, "无法初始化 VPN management 通道。");
            return;
        }
    };
    let mut reader = BufReader::new(reader_stream);
    if writeln!(stream, "{password}\nstate on\nstate\nhold release").is_err() {
        fail(&mut child, state, "VPN management 认证失败。");
        return;
    }
    let mut connecting_since = Some(Instant::now());
    loop {
        if stop.load(Ordering::SeqCst) {
            let _ = writeln!(stream, "signal SIGTERM");
            let until = Instant::now() + Duration::from_secs(3);
            while Instant::now() < until {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            terminate(&mut child);
            set(state, VpnState::Disconnected, "VPN 已断开。");
            return;
        }
        if matches!(child.try_wait(), Ok(Some(_))) {
            set(state, VpnState::Failed, platform::vpn_privilege_guidance());
            return;
        }
        if connecting_since.is_some_and(|t| t.elapsed() > Duration::from_secs(90)) {
            fail(
                &mut child,
                state,
                "VPN 连接超时。请检查网络、证书和系统 TUN / 路由权限。",
            );
            return;
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                fail(&mut child, state, "OpenVPN management 连接已关闭。");
                return;
            }
            Ok(_) => {
                if line.starts_with(">PASSWORD:Need 'Auth' username/password") {
                    let command = credentials
                        .as_ref()
                        .ok_or_else(|| "请输入 VPN 用户名和密码。".to_string())
                        .and_then(Credentials::commands);
                    match command {
                        Ok(command) if stream.write_all(command.as_bytes()).is_ok() => continue,
                        _ => {
                            fail(&mut child, state, "无法提供 VPN 账号密码，请重新连接。");
                            return;
                        }
                    }
                }
                if line.starts_with(">PASSWORD:") || line.contains("Verification Failed") {
                    fail(
                        &mut child,
                        state,
                        "VPN 认证失败，或要求尚未支持的私钥口令/动态验证码。请检查认证方式。",
                    );
                    return;
                }
                if line.starts_with(">FATAL:") || line.starts_with("ERROR:") {
                    fail(
                        &mut child,
                        state,
                        "OpenVPN 报告配置、认证或系统权限错误。请检查配置与平台安装说明。",
                    );
                    return;
                }
                if let Some(s) = management_state(line.trim()) {
                    let message = match s {
                        VpnState::Connected => {
                            connecting_since = None;
                            "OpenVPN 已连接。集群连通性可单独检测。"
                        }
                        VpnState::Reconnecting => {
                            if connecting_since.is_none() {
                                connecting_since = Some(Instant::now());
                            }
                            "OpenVPN 正在重新连接…"
                        }
                        VpnState::Disconnected => "OpenVPN 连接已结束。",
                        _ => "OpenVPN 正在建立隧道…",
                    };
                    if s == VpnState::Disconnected {
                        terminate(&mut child);
                        set(state, VpnState::Disconnected, message);
                        return;
                    }
                    set(state, s, message);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => {
                fail(&mut child, state, "VPN management 通道异常。");
                return;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth_commands_escape_quotes_and_reject_injection() {
        let credentials = Credentials {
            username: "user".into(),
            password: "p\"ass\\word".into(),
        };
        let command = credentials.commands().unwrap();
        assert_eq!(command.lines().count(), 2);
        assert!(command.contains("p\\\"ass\\\\word"));
        assert!(Credentials {
            username: "user\nsignal SIGTERM".into(),
            password: "pass".into()
        }
        .commands()
        .is_err());
    }
    #[test]
    fn state_comes_from_management() {
        assert_eq!(
            management_state(">STATE:123,CONNECTED,SUCCESS,10.0.0.1"),
            Some(VpnState::Connected)
        );
        assert_eq!(
            management_state(">STATE:123,RECONNECTING,transport-error"),
            Some(VpnState::Reconnecting)
        );
        assert_eq!(management_state("Initialization Sequence Completed"), None);
    }
}
