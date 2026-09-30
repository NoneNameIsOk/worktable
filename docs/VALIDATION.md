# 网页版验证（2026-09-30）

本机 macOS arm64。当前是浏览器 + 真实 Axum/SQLite，未重新打包 App。

- `npm run check`：TypeScript、ESLint、15 个前端测试、Vite production build 通过。
- `cargo test --workspace --offline`：12 个共享核心测试 + 4 个 HTTP API 测试通过。
- `cargo clippy --workspace --all-targets --offline -- -D warnings`：通过。
- 保留的桌面壳 `cargo check --manifest-path src-tauri/Cargo.toml --offline`：通过，仅编译兼容检查。
- 浏览器真实操作：中文 Todo 新增、刷新后保留；中文日程新增并出现在首页。不是 mock 页面。
- HTTP 测试：数据库重新打开后记录保留；跨站、伪造 Host、缺失请求头拒绝；VPN 文件内容导入及脚本拒绝；公共配置往返和版本拒绝；桌面启动设置拒绝。
- 提醒事务通过两个数据库连接验证只领取一次。浏览器日期控件自动化未成功保留提醒时间，尚未确认实际页面提醒和系统通知弹出。
- 真实 VPN/TUN/路由、实验室服务登录、VS Code Remote SSH、Windows/Linux 运行尚未验收。CI 已改为网页检查；本地结果不代表远端 CI 通过。

本次浏览器测试数据仅存本机网页数据目录，名称含“网页版验收”，未提交 Git。

---

以下为历史桌面版本验收记录，不能作为当前网页系统能力的验收结果。

# 验证记录

日期：2026-09-15。本机 macOS 15.7.1 / Apple Silicon arm64；Node 24.14.1，npm 11.11.0，项目本地 rustc/cargo 1.98.1。

## 已实际执行

| 检查 | 结果 |
|---|---|
| TypeScript strict typecheck | 通过 |
| ESLint 10 / React Hooks | 通过，无忽略规则 |
| Vitest | 9 / 9 通过（2 文件） |
| Vite production build | 通过 |
| cargo fmt --check | 通过 |
| cargo check --locked | 通过（macOS arm64） |
| cargo test --locked | 11 / 11 通过 |
| cargo clippy --all-targets -- -D warnings | 通过 |
| tauri build --bundles app | 通过，生成 Mach-O arm64 .app |
| npm install audit | 0 vulnerabilities（安装时报告，不等同完整供应链审计） |

前端测试覆盖服务 enabled / requiresVpn、全部 VPN UI 状态、Todo 新增/完成/编辑/删除、失败保存不丢表单、日程新增、sidebar reorder。

Rust 测试覆盖配置 schema/version/敏感字段拒绝、公共导出不含本地隐私数据、非法导入不破坏旧数据、Todo CRUD、提醒去重与重新安排、侧栏完整性、URL/host validation、VPN profile traversal/符号链接拒绝、私有复制 0600、脚本/外部文件/management directive 拒绝、management 状态解析。

## 真实 macOS `.app` 界面验收

已使用原生桌面自动化操作发布 `.app`，不是浏览器 mock：

- 首次打开显示中文 Dashboard、四块内容、空状态和 VPN backend 不可用提示。
- 新增 Todo，编辑为中文，保存后显示；勾选完成后从未完成列表消失。
- 新建中文日程与备注，保存后在首页近期日程显示正确时间。
- 新建本地 HTTP 验收服务，关闭 requiresVpn、设为常用并固定侧栏；侧栏和服务列表同步显示。
- 真正创建 child WebView，页面占据右侧区域，未打开系统浏览器。
- 页面填入中文表单，写入 Cookie/LocalStorage，切首页后再次打开，表单内容、Cookie 和 LocalStorage 均保留。
- 该远程页面主动调用 `get_snapshot`，显示“远程 IPC：拒绝访问（隔离成功）”。
- 点击窗口关闭后窗口隐藏、应用进程仍驻留。

原生验收发现并修复：创建 child WebView 后 `get_webview_window("main")` 不再适用，托盘/第二实例唤起改为 `get_window("main")`；加入 macOS Dock Reopen，系统菜单明确退出也做 VPN 清理。修复后 Rust 测试、Clippy 与 release `.app` 构建通过。

## 尚未完成的验证

后续原生自动化遇到 Mac 锁屏。已请求解锁，但截至本记录未完成以下手动复验；不能把编译通过当作通过这些项目：

- 修复后的托盘重新打开、macOS Dock 重新打开和明确退出；窗口缩放/子视图坐标偏移复验。
- 完整退出并重启后 Cookie/LocalStorage 和本地记录的 UI 验证。SQLite CRUD 和事务已单测通过；之前的服务切换状态保留已原生验证。
- OS 通知实际弹出、Tray 内提醒、autostart 在登录后的效果；提醒查询去重已单测。
- 原生文件对话框导入导出往返、VPN 原始文件移动后真实 UI 使用；相关路径/私有复制/配置导入导出已单测。
- 真实 VPN 连接、驱动/helper/TUN/路由/DNS：未提供 binary 和真实配置，没有执行。
- 真实 ClearML/New API session、OAuth、VS Code Remote SSH 和集群节点检测：未连接实验室 VPN，没有执行。
- Windows、Ubuntu、macOS 13：本机不能运行，CI 配置尚未提交执行。
- 签名、公证、Windows installer、Linux AppImage/DEB 和 macOS DMG 分发：本次仅验收未签名 `.app` 构建。

## 可重复 WebView 验收

运行 `node scripts/webview-fixture.mjs`，在桌面服务中添加 `http://127.0.0.1:17832`，requiresVpn 关闭。页面提供表单、持久化测试存储和无本地权限的 IPC 探测。此服务仅绑定本机且无真实数据。完成后停止脚本，按需删除自己创建的测试入口。

本次手动验收创建了明确命名的 Todo、日程和 `WebView 验收` 服务记录。它们只在本机应用数据目录，不是工程预置数据。锁屏期间没有擅自清空用户数据目录。
