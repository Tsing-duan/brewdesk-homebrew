# 源码检查与构建


需要 Node.js 24、`rust-toolchain.toml` 指定的 Rust 工具链，以及项目本地的 `@tauri-apps/cli` 2.11.4。使用锁文件安装依赖。

构建输出应放在仓库外的私有目录。下面是仓库现有的源码检查步骤，请先把 `/absolute/private/brewdesk-build` 改成自己的绝对路径：

```bash
install -d -m 700 /absolute/private/brewdesk-build
export PUBLIC_BUILD_ROOT=/absolute/private/brewdesk-build
export CARGO_TARGET_DIR="$PUBLIC_BUILD_ROOT/cargo-target"
npm ci
npm test
npm run build
npm run test:rust-output-boundary
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --locked --manifest-path src-tauri/Cargo.toml --all-targets --all-features
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

`PUBLIC_BUILD_ROOT` 必须位于仓库外，否则 `scripts/build-native-helpers.sh` 会拒绝运行。Rust、Swift、Vite、Tauri 和 `.app` 产物使用该外部构建目录；`test:rust-output-boundary` 检查 Tauri capability schemas 也没有写入仓库的 `src-tauri/gen`。

应用标识为 `io.github.tsing-duan.brewdesk`。本地 ad-hoc 签名检查不等于通过 Gatekeeper 或完成 Apple 公证。

## 参与开发

请先阅读[贡献指南](../CONTRIBUTING.md)和[仓库开发说明](../AGENTS.md)。
