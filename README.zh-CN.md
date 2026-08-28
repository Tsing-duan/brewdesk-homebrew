# BrewDesk

> 面向中文用户的非官方 Homebrew macOS 桌面客户端：透明显示实际命令，识别其他来源应用的安装冲突，并只对当前任务应用代理或镜像配置。

[English](README.md)

BrewDesk 是独立社区项目，不由 Homebrew 开发、认可或支持。

## 截图

![BrewDesk 公开 Alpha 搜索界面](docs/screenshots/brewdesk-search.png)

该截图来自最终 Stage A `.app` 构建，并已按完整尺寸检查私人路径、软件清单、测试者或接收者信息、凭据、私有命令及无关应用。参见[截图捕获指南](docs/screenshots/CAPTURE_GUIDE.md)。

## 为什么有 BrewDesk

Homebrew 透明而强大，但终端优先的工作方式对部分中文 macOS 用户并不熟悉。BrewDesk 提供本地桌面界面，同时继续显示真实 Homebrew 操作并保留其命令行行为。

## 已实现行为

- 使用英文名称、中文名称和本地维护的中文别名搜索 Formula 与 Cask。
- 显示包详情、已安装版本、依赖、Homebrew 输出和操作阶段。
- 在安装、卸载、升级或修复前预览真实 Homebrew 命令。
- 检查 Cask 声明的应用路径；其他来源应用占用目标时给出警告。
- 在 macOS 提供相关证据时识别 Mac App Store receipt。
- 所有 Homebrew 变更操作进入单 worker 队列，避免并发执行。
- 先显示本地可更新缓存，再在后台刷新。
- 系统代理、直连或镜像变量只应用于当前子任务。

## 安全边界

- Homebrew 操作使用“可执行文件 + argv 数组”，执行前验证包 token。
- 搜索文本允许 UTF-8；会改变系统状态的目标使用严格 token 语法。
- BrewDesk 不把代理或镜像设置写入 Homebrew、macOS 或 Shell profile。
- BrewDesk 不采集 sudo 密码；交互式 Homebrew 安装交由 Terminal 完成。
- 其他来源检测采用保守策略，无法可靠区分所有 DMG、PKG、手动复制或管理系统安装。
- 不包含遥测、账号、分析 SDK 或软件清单上传。

详见[安全模型](docs/SECURITY_MODEL.md)和[网络路由](docs/NETWORK_ROUTING.md)。

## 平台合同

当前验证目标为 Apple Silicon（`arm64`）与 macOS 26。这不能证明支持 Intel Mac、macOS 14 或 macOS 15。

公共构建合同使用：

- Node.js 24
- npm lockfile安装
- `rust-toolchain.toml` 中固定的精确 Rust toolchain
- 项目本地 `@tauri-apps/cli` 2.11.4

## 源码验证

```bash
npm ci
npm test
npm run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Rust、Swift、Vite、Tauri 和 `.app` 产物必须定向到仓库外私有构建根。未设置 `PUBLIC_BUILD_ROOT` 或其位于仓库内时，`scripts/build-native-helpers.sh` 会拒绝运行。

最终 Bundle Identifier 已确认为 `io.github.tsing-duan.brewdesk`。应用图标由 Codex 专门为 BrewDesk 生成，其来源记录见[素材来源](docs/ASSET_SOURCES.md)。Ad-hoc 验证只证明源码构建完整性，不代表 Gatekeeper、公证或二进制分发批准。

## 测试

Stage A 源码基线为前端 16 项、Rust 30 项。最终数量可以增加，但不得低于重新核验的真实基线。Skipped、ignored、filtered、todo 或空壳测试均不计数。

## 已知限制

- 当前界面和用户体验主要在中文环境中验证。
- 网络探活只覆盖有限的 Homebrew、GitHub、registry 和镜像端点；第三方 Cask 下载站仍可能独立失败。
- 应用来源分类基于可观察证据，但不完整。
- 当前 Alpha 不提供已签名或公证的公开二进制。
- 二进制分发仍未批准，需另行完成版本对应的第三方许可声明包、签名、公证和分发批准。

详见[已知限制](docs/KNOWN_LIMITATIONS.md)和[路线图](docs/ROADMAP.md)。

## 贡献与支持

提出修改前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)、[AGENTS.md](AGENTS.md) 和 [SUPPORT.md](SUPPORT.md)。安全问题按 [SECURITY.md](SECURITY.md) 处理。

## 许可证

BrewDesk 源码采用 [MIT License](LICENSE)。第三方依赖及单独授权材料继续适用其各自条款，参见 [NOTICE.md](NOTICE.md) 和[素材来源](docs/ASSET_SOURCES.md)。

## 独立项目声明

Homebrew 及相关名称属于各自权利人。BrewDesk 不声称官方身份、认可、合作关系或超出现有证据的平台兼容性。
