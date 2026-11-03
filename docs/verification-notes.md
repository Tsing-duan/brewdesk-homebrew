# 验证记录


早期 Stage A 源码验证基线为前端 16 项、Rust 30 项；Release 记录了该版本的测试结果。这里保留的是已有记录，并非新一轮测试结果。跳过、忽略、过滤、待办或空测试不计入基线。

截图来自实际 `.app` 构建，相关说明见[截图指南](screenshots/CAPTURE_GUIDE.md)。应用图标使用 Codex 生成，见[素材来源](ASSET_SOURCES.md)。

- [路线图](ROADMAP.md)
- [贡献指南](../CONTRIBUTING.md)与[仓库开发说明](../AGENTS.md)
- [问题反馈](../SUPPORT.md)与[安全问题报告](../SECURITY.md)


## 发行前仍需完成的工作

目前没有公开安装包。发布二进制前仍需整理对应版本的第三方许可声明，并完成 Developer ID 签名和 Apple 公证。许可要求见 [NOTICE.md](../NOTICE.md)。

## 实现与适用范围

Homebrew 操作使用可执行文件与独立参数数组，变更目标先做严格校验，搜索文本支持 UTF-8。应用来源检测及网络检查的范围见[安全模型](SECURITY_MODEL.md)、[网络设置](NETWORK_ROUTING.md)和[已知限制](KNOWN_LIMITATIONS.md)。

## Source baseline details


- Homebrew actions are represented as an executable plus an argv array; package tokens are validated before execution.
- UTF-8 is accepted for search text, but mutation targets use a strict token grammar.
- BrewDesk does not persist proxy or mirror settings to Homebrew, macOS, or shell profiles.
- BrewDesk does not collect sudo passwords. Interactive Homebrew installation is delegated to Terminal.
- Other-source application detection is conservative. It cannot reliably distinguish every DMG, PKG, manual copy, or management-system installation.
- No telemetry, account system, analytics SDK, or software-inventory upload is included.

More detail is available in [Security Model](SECURITY_MODEL.md) and [Network Routing](NETWORK_ROUTING.md).


The Stage A source baseline is 16 frontend tests and 30 Rust tests. Final counts may increase but must never fall below the re-verified baseline. Skipped, ignored, filtered, todo, or empty tests do not count.

