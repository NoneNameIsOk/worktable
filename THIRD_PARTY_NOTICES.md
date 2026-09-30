# Third-party notices

本项目界面和业务实现为本任务新编写，没有复制 Homarr、Dashy、Heimdall 或其他 Dashboard 源码。以下是直接依赖的主要许可信息；精确版本以 package-lock.json 和 src-tauri/Cargo.lock 为准。分发时应包含锁定版本完整 dependency license notices，包括传递依赖。

| 组件 | 上游 | 许可 |
|---|---|---|
| Tauri、官方 autostart/notification/single-instance 插件 | https://github.com/tauri-apps | MIT OR Apache-2.0 |
| React / React DOM | https://github.com/facebook/react | MIT |
| Vite / Vitest | https://github.com/vitejs | MIT |
| TypeScript | https://github.com/microsoft/TypeScript | Apache-2.0 |
| dnd-kit | https://github.com/clauderic/dnd-kit | MIT |
| Lucide | https://github.com/lucide-icons/lucide | ISC；部分图标来源另有 MIT notices，随包保留其 LICENSE |
| rusqlite | https://github.com/rusqlite/rusqlite | MIT |
| SQLite | https://www.sqlite.org/copyright.html | Public domain |
| serde / serde_json、uuid、chrono、reqwest、url、semver | 各 crate Cargo.toml 的 repository | MIT OR Apache-2.0（按每个锁定 crate 的声明保留） |
| rfd | https://github.com/PolyMeilex/rfd | MIT |

## OpenVPN：尚未随包分发

官方来源：https://github.com/OpenVPN/openvpn/blob/master/COPYING 。2026-09-15 核对该文件：OpenVPN distributed under GPL version 2，有 OpenSSL 和 Apache2 linking exceptions；完整条件以具体分发版本 COPYING 为准。OpenVPN 为 OpenVPN Inc 商标。

此仓库不包含 OpenVPN binary、源码、驱动和私钥。未来 binary 分发必须附带该版本完整许可、版权声明、对应源码/合规源码提供方式和所有修改记录，并单独核对 Wintun/TAP/OpenSSL 等依赖条款。不可用这份摘要代替正式随包许可文件。

本地 `source.svg` 应用图标为本项目新绘制，不使用第三方图像。
