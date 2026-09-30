# 网页架构与数据边界

当前主入口为本机网页。React 的 `src/lib/api.ts` 按运行环境选择同源 HTTP 或 Tauri invoke，业务逻辑共用 `crates/worktable-core`。

```text
浏览器 React
  → /api/<command> POST JSON
  → Axum 127.0.0.1:1421
      → SQLite / VPN profile / OpenVPN manager / VS Code / UpdateService
开发：Vite 127.0.0.1:1420 代理 /api
构建后：Axum 直接提供 dist 静态文件
```

HTTP 只接受本机两个端口的 Host/Origin，要求自定义请求头，拒绝跨站 Fetch；没有 CORS。请求体有大小限制，参数采用类型化反序列化，错误返回 JSON。响应禁止缓存 API，设置 CSP、nosniff、禁止 frame 和 referrer。没有提供任意文件读取、shell 执行或桌面自启动 API。

服务使用校验后的 HTTP(S) 链接，由用户点击打开新标签页，不依赖 iframe。需要 VPN 的服务和节点在 Worktable 隧道已连接或集群健康检测成功时可打开，因此兼容已有外部 VPN。健康检测不代表 Worktable 建立了隧道。

网页日程每 15 秒轮询；SQLite immediate 事务查询和标记提醒，多个页面只领取一次。只在页面可见或已有通知权限时领取。领取后页面崩溃可能漏提醒，关闭页面没有后台 daemon。桌面专用的 autostart/tray/VPN 自动连接设置在网页隐藏且 API 拒绝修改。

网页使用 `org.hyksj.worktable-web` 独立数据目录。上传 `.ovpn` 经校验后私有保存，配置包导出只含公共类型。公共配置导入由浏览器确认后事务替换。旧桌面数据不自动复制。

以下为保留的桌面架构记录；其中原生 WebView、托盘、OS 通知生命周期仅适用于后续桌面模式，不适用于当前网页。

---

# 架构与数据边界

HYKSJ Worktable 是本地桌面启动台。React 呈现工作区，Rust 执行受控系统能力；没有账号服务器、内置 IDE/Terminal、实验后端或插件运行时。

```text
主 WebView：React / TypeScript
    ↓ typed invoke（commands.rs 校验 main 标签及本地 origin）
Rust AppState
    ├── Mutex<Database> → SQLite WAL / v1 migration
    ├── VpnProfileManager → 私有文件目录
    ├── VpnManager → BundledOpenVpnBackend → OpenVPN management
    ├── ServiceViews → 独立远程 child WebViews
    ├── platform → VS Code / URL / 权限能力说明
    └── notification loop / Tray / autostart / UpdateService
远程 WebViews → 各自网站（没有本地 IPC 能力）
```

## 前端

`app/` 布局与状态编排，`components/` 表单、侧栏与对话框，`modules/` 业务界面，`lib/api.ts` 为类型化 IPC 接口，`types/` 是可序列化数据模型。状态来自 Rust snapshot；成功写入后刷新，不在 localStorage 中存业务数据或 VPN 内容。单个应用无需引入额外全局 store 框架。

侧栏由 `sidebar_items` 驱动，dnd-kit 支持指针和键盘拖动。模块 ID 不可删除，固定服务使用 `service:<uuid>`。后端校验重排集合完整。服务与节点通过上下移动排序。

测试中的 mocks 仅位于测试文件。网页实际使用 Axum 和 SQLite。

## SQLite

`app_settings` 为独立类型化列；其余为 `vpn_profiles/services/nodes/todos/calendar_events/sidebar_items`。通过 `PRAGMA user_version` 执行首次事务迁移，更新版本数据库拒绝旧应用打开。所有 SQL 值使用绑定参数，服务/节点写入和 pin 同步在同一事务中完成。

首次仅创建 vipsl1–15 和内置侧栏；不写入虚构服务地址。SQLite 位于系统 application data 下的 `org.hyksj.worktable` 目录。macOS 通常为 `~/Library/Application Support/org.hyksj.worktable/`。VPN 私钥只在其 `vpn/` 私有目录，不写入 SQLite，也不通过 IPC 返回内容。

实验室配置由单独的 `LabConfig`/`PublicDefaults` 类型导出，不从完整 snapshot 挑字段。serde `deny_unknown_fields`、类型、UUID、版本、URL、数量、重复 ID 全量验证后，事务替换公共服务与节点；保留本地 Todo、日程、VPN、autostart、VPN autoconnect。导入前原生对话框说明覆盖范围。

服务入口和公共 URL 限制为无用户信息、query、fragment 的 HTTP(S)，防止复制带登录凭证的 URL 到公共包。备注/名称由管理员确认只放公共信息，工具无法判断自然语言是否是秘密。

## WebView

Tauri 2.11 的 child WebView API 仍需要 `unstable` feature，版本由 Cargo.lock 固定。不使用 iframe。主窗口内只有一个可见服务视图；已打开视图缓存在 `ServiceViews`，切换只 hide/show，保留 DOM、URL、表单和 session。修改入口 URL/安全开关、删除服务、导入新配置包、点击“重新打开”才关闭旧视图。服务改名/常用/pin/排序不重建页面。

Windows/Linux 为每个服务 UUID 设置独立持久化 `web-data/<id>` 目录。macOS 14+ 用 UUID 作为 WKWebsiteDataStore identifier；macOS 13 回落默认持久 store，按 origin 隔离，不能承诺同一 origin 的不同服务账号隔离。不会手工读取或同步 cookies，也不保存服务密码。

远程视图只允许 HTTP(S) 导航，新窗口请求拒绝。因此某些 OAuth popup、下载弹窗、第三方身份验证流程尚不支持。远程网站自身可能强制退出、禁用 session 持久化或遇到证书错误，应用不绕过这些限制。跨站登录真实性需在实验室服务上验收。

`capabilities/main.json` 仅匹配本地主视图，没有 `remote` URL 权限。所有自定义命令额外检查 `label=main` 和本地 origin；远程 WebView 即使探测到 Tauri runtime 标识也无法调用数据库、VPN、节点、文件、更新命令。只给主界面 `core:default`，没有 shell/fs/sql 插件权限。主界面采用 CSP，不加载 CDN 脚本。

## 后台生命周期

单实例插件防止重复运行提醒线程/VPN。窗口关闭隐藏到 Tray。Tray 提供打开、状态、连接上次 VPN、断开和退出；窗口关闭事件只隐藏。明确选择 Tray 退出或系统菜单退出后清理 VPN 并退出；macOS Dock Reopen 和第二实例均通过原生 Window 句柄重新显示，即使已存在子 WebView。系统强杀/崩溃不具备正常清理保证。

Rust 每 15 秒检查已到提醒时间且未结束的日程，通知成功后记 `notified_at`。编辑备注不会重发；改变开始/提醒时间会重置。重新启动会补发尚未结束的到期日程，已结束日程不补发。应用完全退出时没有后台 daemon 提醒。发送成功到数据库记录之间若崩溃可能重发一次，通知是系统投递成功语义，不是用户已读语义。

autostart 和 VPN autoconnect 独立且默认关闭。更改 autostart 先调用 OS 注册，再持久化；数据库失败会尝试恢复之前的 OS 注册状态。启动进 Tray 独立可选。通知权限由设置页面主动申请。

## 节点与更新

VS Code 检测二进制和 `ms-vscode-remote.remote-ssh`，再用参数数组 `--new-window --remote ssh-remote+<validated-host>` 启动。Windows 直接调用 Code.exe + cli.js，避免经过 cmd.exe/.cmd。不会修改 SSH config 或密钥。连通性检测是直接 DNS + TCP 22，不执行 SSH。

UpdateService 读取用户设置的 HTTPS JSON endpoint，验证 SemVer、大小和下载 URL，展示发布说明；仅用户点击下载才调用系统浏览器。不自动下载、安装或执行更新文件。
