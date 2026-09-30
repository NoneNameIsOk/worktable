# OpenVPN 发布资源目录

当前不含任何 OpenVPN binary、驱动或 helper。禁止放入来源不明的二进制。

`BundledOpenVpnBackend` 查找 Tauri resource_dir 下的：

- `binaries/macos/aarch64/openvpn`
- `binaries/windows/x86_64/openvpn.exe`
- `binaries/linux/x86_64/openvpn`

准备审核过的二进制、运行时库、GPL 源码/许可证材料及平台 helper 后，在发布专用 Tauri 配置的 `bundle.resources` 中将该平台目录映射到上述相对位置（参见 `docs/VPN.md`）。此目录布局是资源型 sidecar；Rust 使用绝对路径和参数数组启动，不依赖 shell plugin。

开发 debug 构建允许系统 OpenVPN fallback。release 构建不使用 PATH 中的 OpenVPN，缺少随包 binary 时明确报告不可用；Worktable 的其他功能仍可构建、运行。
