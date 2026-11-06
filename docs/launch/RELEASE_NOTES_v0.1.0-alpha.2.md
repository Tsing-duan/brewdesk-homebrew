# BrewDesk v0.1.0-alpha.2 — 更新可靠性与界面维护

这是源码预发布版本，提供 GitHub 自动生成的源码归档，没有 `.app`、DMG 或 PKG 安装包。

## 更新内容

- 更新后核对 Homebrew 安装记录和应用证据，改善下载进度解析与任务级网络路由。
- 窄窗口导航保留文字标签；详情支持返回列表并保留结果与滚动位置。
- 设置优先展示状态和下载线路，高级路径与可选翻译按需展开。
- 更新兼容的 React、Tauri 和 Rust 依赖，包含 Tokio I/O 安全修补。
- 同步中英文介绍、验证记录与真实原生应用截图。

## 验证

实现提交 `ec1e9330229ca53025ddb95652406b81e3e56286` 通过 35 项前端测试、43 项 Rust 测试、外部应用构建、签名结构检查和秘密扫描。

- [实现提交的 CI（三项成功）](https://github.com/Tsing-duan/brewdesk-homebrew/actions/runs/42214098425)
- [验证范围](https://github.com/Tsing-duan/brewdesk-homebrew/blob/v0.1.0-alpha.2/docs/verification-notes.md)
- [维护说明](https://github.com/Tsing-duan/brewdesk-homebrew/blob/v0.1.0-alpha.2/docs/MAINTENANCE_2026-11.md)

npm 与 RustSec 审计均为零漏洞条目；Rust 仍保留 `proc-macro-error` 停止维护及 `glib` unsoundness 两项信息性警告。CI 在 macOS 26 上运行；公开截图来自 macOS 27.0.1 / arm64 的真实应用，截图只证明所展示的界面状态。

## 限制

- 暂无 Developer ID 签名、公证或公开安装包。
- 真实批量升级、历史隐私权限提示、完整可访问性和全部应用启动仍需进一步验收。
- 不宣称 Intel、macOS 14/15 或所有其他系统版本已兼容。
- 应用内部版本仍为 `0.1.0` / build `1`；源码标签用于区分发布快照。
- 更新源码不会自动替换本机旧应用。

## English summary

Source-only prerelease with update-result verification, task-scoped network routing,
compact navigation and detail-return fixes, progressive disclosure in Settings,
and compatible dependency updates. The implementation passed 35 frontend and
43 Rust tests plus all three CI jobs. Native screenshots and bilingual documentation
are refreshed. Two Rust advisory warnings remain disclosed. No application binary,
notarization, universal accessibility conformance, or complete real-upgrade
acceptance is claimed.
