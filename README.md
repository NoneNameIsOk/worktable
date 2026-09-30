# HYKSJ Worktable

科研课题组的一站式本地网页工作台。当前优先完善网页版，之后再打包 App。React + TypeScript strict + Rust/Axum + SQLite；已有 Tauri 适配层保留，当前不制作桌面安装包。

## 启动网页版

需要 Node.js 22.12+、npm、Rust stable 和系统 C/C++ 构建工具。本机已验证 Node 24 / Rust 1.98 / macOS arm64。

```sh
npm ci
npm run dev
```

打开 http://127.0.0.1:1420 。一条命令编译并启动本机 HTTP 后端和 Vite，业务数据真实写入 SQLite。不要直接打开 `index.html`。停止命令会同时关闭前后端。

运行构建后的网页：

```sh
npm run build
npm start
```

此时访问 http://127.0.0.1:1421 。后端只提供 `dist/` 静态资源，不暴露源码或数据目录。`npm start` 仍编译本机 debug 后端，不是 App 打包。

启动脚本自动使用已有项目本地 `.tools` Rust；其他开发者直接使用自己的 cargo。独立运行 Rust 检查时，若 PATH 无 cargo，可先执行 `. scripts/rust-env.sh`。不需要复制或提交 `.tools`。

## 功能

- **首页**：网络状态、近期日程、Todo、常用服务。
- **Todo / 日程**：新增、编辑、完成、删除与本地持久化；日程支持可选提醒。
- **服务**：CRUD、排序、常用、固定侧栏、启用、VPN 要求。通过后端验证后在浏览器新标签页打开，登录状态由浏览器和服务管理。
- **侧栏**：拖拽或键盘调整，顺序持久化。
- **节点**：预置 vipsl1–15 名称，可编辑、排序；后端可检测 TCP 22、调用已安装的 VS Code Remote SSH，不修改 SSH 配置或密钥。
- **VPN**：自行选择或拖拽上传自包含 `.ovpn`（支持多文件，每个不超过 2 MB，逐项显示结果）、管理多个配置、调用本机 OpenVPN。也可使用已有 VPN 客户端；集群可达与 Worktable VPN 状态分别显示。
- **设置**：实验室名称、公共配置包导入/导出、通知权限、版本检查。配置导入前确认覆盖服务与节点，保留 Todo、日程和 VPN。

网页不显示托盘、开机启动等桌面选项；实验与插件仍明确标注为占位，未实现业务后端。

## 数据与运行边界

网页数据位于系统数据目录的 `org.hyksj.worktable-web`，macOS 为 `~/Library/Application Support/org.hyksj.worktable-web/`。可用 `WORKTABLE_DATA_DIR` 指定独立目录。旧桌面数据目录 `org.hyksj.worktable` 保留，不自动迁移或共用。不要让多个后端共享同一数据目录。

这是**本机单用户网页版**，仅监听 127.0.0.1。没有账号和多人权限体系，不是公网部署版本。API 验证 Host、Origin、自定义请求头和跨站请求，禁止跨域访问；VPN 内容只写入本机私有文件，不写入前端存储或公共配置包。

日程提醒需要网页和后端保持运行，每 15 秒检查。多个标签页通过数据库事务避免重复领取。关闭网页、浏览器休眠或领取后崩溃可能漏提醒；系统通知需浏览器权限，页面内提醒不需要该权限。

本仓库没有附带 OpenVPN binary、驱动或特权 helper。当前 debug 后端可使用已安装的系统 OpenVPN，缺少时明确显示不可用。真实 VPN、实验室服务登录及 VS Code 集群连接尚未验收。不自动安装驱动、更改系统权限或 SSH 配置。详见 [VPN 说明](docs/VPN.md)。

## 检查

```sh
npm run check
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --locked --all-targets -- -D warnings
```

CI 检查前端与 Windows/macOS/Linux 的 Rust 网页后端，不打包 App。实际执行记录见 [验证记录](docs/VALIDATION.md)。

## 目录

```text
src/                       React 界面、HTTP/Tauri 适配、前端测试
crates/worktable-core/      SQLite、模型、配置校验、VPN、平台能力
server/                    本机 HTTP API 与静态资源服务
scripts/web.mjs            前后端启动与退出
src-tauri/                 保留的桌面适配层，后续打包时复用
examples/                  公共配置示例（example.org 需替换）
docs/                      架构、VPN、验证记录
```

[架构与安全边界](docs/ARCHITECTURE.md)
