# 验证记录

## 2026-11 源码预发布

本次实现提交为 [`ec1e933`](https://github.com/Tsing-duan/brewdesk-homebrew/commit/ec1e9330229ca53025ddb95652406b81e3e56286)。后续发布资料提交仅更新文档和截图，不改变该实现。

| 检查 | 结果与范围 |
| --- | --- |
| 前端测试 | 35 项通过，无跳过 |
| Rust 测试 | 43 项通过，无忽略或过滤 |
| 源码检查 | TypeScript/Vite、Rust fmt、Clippy、shell/CI 合约通过 |
| 构建边界 | 构建产物与 Tauri 生成文件位于仓库外 |
| 原生应用 | arm64 `.app` 构建、Info.plist、ad-hoc 签名结构检查通过 |
| 秘密扫描 | 动态 Canary、公开源码与完整 Git 历史扫描通过 |
| npm 审计 | 零漏洞条目 |
| RustSec 审计 | 零漏洞条目；保留两项信息性警告，见下文 |

[实现提交的 GitHub CI](https://github.com/Tsing-duan/brewdesk-homebrew/actions/runs/42214098425) 三项任务全部通过。CI 使用 macOS 26；最新发布提交的 CI 可从 [Actions](https://github.com/Tsing-duan/brewdesk-homebrew/actions) 查看。

Rust 警告仍包括：`proc-macro-error` 1.0.4 停止维护（RUSTSEC-2024-0370），以及 `glib` 0.18.5 的 unsoundness 警告（RUSTSEC-2024-0429）。不能将零漏洞条目理解为没有任何安全提示。依赖变化见[维护说明](MAINTENANCE_2026-11.md)。

## 界面证据

- 浏览器模拟预览检查了搜索、详情返回和设置，覆盖 1053、1440、390 CSS 像素宽度；不代表真实 Homebrew 执行。
- 2026-11-06 从上述提交的真实 `.app` 拍摄搜索与设置页，运行环境是 macOS 27.0.1 / arm64。确认了应用启动及这两个界面的可见状态；这不等于完成 macOS 27 的全面兼容性验收。
- 两张公开截图已逐张检查，未展示用户路径、软件库存、操作历史或凭据。转换为 PNG 并去除 EXIF/XMP 元数据；来源与哈希见[素材来源](ASSET_SOURCES.md)，流程见[截图指南](screenshots/CAPTURE_GUIDE.md)。

## 尚未完成的验证

真实批量升级、更新后的所有应用启动、历史隐私权限提示、完整 VoiceOver 流程、全部对比度和交互状态仍未完成验收。当前截图和测试不能支持“所有设计规范都符合”的结论。

暂无公开安装包。二进制发布还需要对应版本的第三方许可材料、Developer ID 签名、Apple 公证和单独的发布授权，见 [NOTICE](../NOTICE.md)。

## 历史基线

早期 Stage A 的基线为前端 16 项、Rust 30 项；历史资料保留在[Stage A 审计摘要](AUDIT_SUMMARY.md)和首个源码 Release。它们不代表本轮最新测试数量。跳过、忽略、过滤、待办或空测试不计入通过数量。
