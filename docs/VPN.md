# OpenVPN：实现、平台能力与发布工作

## 当前状态

已实现配置管理、私有副本、多 profile、最后使用记录、状态机、进程启动/清理、Management Interface 和 capability check。当前仓库没有随包 OpenVPN、Wintun/TAP 驱动或 privileged helper。不能将本版本称为三平台开箱即用 VPN 客户端。

当前开发机 macOS Apple Silicon：未提供真实实验室 .ovpn 和可用 OpenVPN binary，所以没有进行真实隧道、路由、DNS 或集群连接验收。不会读取用户已有 VPN 私钥来进行未经要求的连接。

## 导入与敏感数据

通过系统文件对话框导入小于 2 MB 的 UTF-8 `.ovpn`。源路径 canonicalize，只接受普通文件。生成 UUID 后复制到应用数据的 `vpn/<uuid>.ovpn`，不保存源路径。Unix 目录 0700、文件 0600；Windows 目录/文件使用当前用户 ACL，ACL 失败会取消导入。使用 UUID 构造路径，拒绝 traversal 和符号链接。

支持可信实验室 self-contained 文件。证书与 key 必须内联。preflight 使用正向 directive 列表：允许常见 client、remote、协议、TLS/cipher、路由、keepalive 等参数；仅允许 ca/cert/key/tls-auth/tls-crypt/tls-crypt-v2/peer-fingerprint 内联区块。脚本、plugin、外部文件、config include、daemon、log、management 覆盖、跨行 directive 等不接受。具体列表见 `crates/worktable-core/src/vpn/profile.rs`。

不支持外部 ca/cert/key、auth-user-pass、加密私钥交互口令。未知 directive 会要求管理员整理配置，不会默默剔除。配置预检不替代 OpenVPN 对证书和协议的校验。私钥、配置正文、management 密码和原始进程输出不记录到普通日志或前端。

## Backend 与 Management

`VpnBackend` 定义 executable/capability；`BundledOpenVpnBackend` 优先查找资源目录固定路径，debug 构建才允许 system fallback。`VpnProfileManager` 管理私有文件；`VpnManager` 持有唯一 worker、进程和 stop 信号。

连接过程：验证配置 → 创建随机 management 密码 0600 临时文件 → 只绑定 127.0.0.1 → 使用 Command + 独立参数启动 OpenVPN → 认证 management → `state on` / `state` / `hold release`。使用 `>STATE` 通知驱动 Disconnected / Connecting / Connected / Reconnecting / Disconnecting / Failed，不依赖 stdout 搜索。

为了兼容 OpenVPN TCP management server，应用先选择空闲本地端口后释放，由 OpenVPN 绑定。端口竞争会明确失败，随机密码保护 management。密码文件在 worker 结束时删除；系统强杀可能留下仅当前用户可读的临时文件。进程 stdout/stderr 丢弃，不泄露内联材料。首次 management 连接超时 20 秒，隧道建立/重连超时 90 秒。连接失败显示配置/认证/权限检查指引，不把任何含密钥的原始输出传给用户。

断开先通过 management 发送 SIGTERM，给 OpenVPN 清理路由的机会，3 秒后仍未结束才清理本应用创建的 child 并 wait。仅管理自己启动的进程。

OpenVPN Connected 与 `clusterHealthCheckUrl` 的 HTTP 成功分别呈现。HTTP 检测直接连接、禁用代理，失败不改 VPN 状态。没有真实 VPN 时不使用 health check 冒充 Connected。

## 三平台权限方案

| 平台 | 后端能力边界 | 发布前仍需完成 |
|---|---|---|
| Windows 10/11 x64 | 固定路径 OpenVPN.exe；当前用户 ACL；capability 可读提示 | 签名 installer 安装 Wintun/TAP 和受限 Windows Service，命名管道限制当前用户，service 仅接受已导入 profile ID；验证清理路由与卸载 |
| macOS 13+ arm64 | 系统/随包 OpenVPN；0700/0600；WKWebView | 签名 notarized app 与 privileged helper，采用适用的 SMAppService/授权安装机制，XPC 校验调用者签名，helper 只操作限定配置；验证 utun、DNS、睡眠恢复 |
| Ubuntu 22.04/24.04 x64 | 系统/随包 OpenVPN；/dev/net/tun；0700/0600 | 安装受限 polkit + root helper/systemd service，校验调用者与 profile ID；验证 NetworkManager/systemd-resolved 兼容与路由清理 |

这些 helper 是明确未实现的独立工作项；当前 capability 不假装已获得特权，也不会自动 sudo、UAC、修改 sudoers、setuid、安装驱动或改变系统网络配置。不要把整个 WebView 应用以 root 身份运行。开发 fallback 仅在系统已具备适当权限环境时才可能真正连通。

## 可信二进制打包

准备经过来源和许可证审核的 OpenVPN 与依赖后，将文件放入对应目录：

```text
src-tauri/binaries/macos/aarch64/openvpn
src-tauri/binaries/windows/x86_64/openvpn.exe
src-tauri/binaries/linux/x86_64/openvpn
```

在平台发布配置中映射资源，例如 macOS `bundle.resources`：

```json
{
  "bundle": {
    "resources": {
      "binaries/macos/aarch64/": "binaries/macos/aarch64/"
    }
  }
}
```

连同平台依赖库、签名 helper、驱动和许可证材料一起打包，不从不可信来源自动下载。发布 binary 必须确保 executable 权限、依赖装载路径和签名正确。

OpenVPN 官方 COPYING 标明 GPL v2，并有 OpenSSL / Apache2 linking exceptions。将 CLI 作为独立进程随包发布仍需满足它及其依赖/驱动的分发义务：完整许可、copyright、对应源码或适当源码提供方式、修改记录。Worktable 没有复制 OpenVPN/GPL Dashboard 源码。不要因为进程独立就忽略 CLI 本身的 GPL 分发要求。参见 `THIRD_PARTY_NOTICES.md`。

## 网页账号认证

支持无参数 `auth-user-pass`，连接时输入账号密码，通过本机 OpenVPN management 通道提供，仅保留于当前连接进程内存，不写入数据库或凭据文件。支持 `<connection>` 内的受控连接选项。外部凭据文件、交互式私钥口令、动态验证码仍不支持。协议参考：https://openvpn.net/community-docs/management-interface.html 。
