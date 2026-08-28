# Repository Instructions for Coding Agents

These instructions apply to the entire BrewDesk repository.

## Read first

Read `README.md`, `CONTRIBUTING.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY_MODEL.md`, `docs/NETWORK_ROUTING.md`, and `docs/KNOWN_LIMITATIONS.md` before modifying behavior.

## Safety boundaries

- Construct external commands as an executable plus an argv array. Do not create shell command strings by concatenating user-controlled text.
- Search text may be Unicode. Tokens used by install, uninstall, upgrade, repair, or other mutations must pass the strict package-token validator.
- Apply proxy and mirror variables only to the child process for the current task. Never write global Homebrew, macOS, Git, npm, Cargo, or shell configuration.
- Do not add telemetry, analytics, accounts, cloud inventory upload, or password collection.
- Preserve conservative Formula/Cask behavior, application-origin evidence, Mac App Store receipt handling, and application conflict protection.
- Preserve the single-worker mutation queue unless a separately approved design changes that contract.
- Treat external URLs, package metadata, process output, and filesystem state as untrusted input.

## Build and verification

- Use Node.js 24 and the exact dependencies in the lockfile.
- Use the exact Rust toolchain in `rust-toolchain.toml` and the project-local exact `@tauri-apps/cli`.
- Keep all Rust, Swift, Vite, Tauri, and `.app` outputs under an absolute `PUBLIC_BUILD_ROOT` outside the repository.
- Use an external `CARGO_TARGET_DIR` and run `npm run test:rust-output-boundary`; Tauri capability schemas must remain in the external build context, never `src-tauri/gen`.
- Do not chain shell commands into opaque one-liners in project scripts. Fail explicitly and preserve the first actionable error.
- Check official sources before updating tool versions, Actions commit SHAs, platform claims, or security-advisory conclusions.
- Add focused tests for package-name injection, argv construction, task-scoped network variables, global-write prohibitions, application conflict handling, and queue behavior when those areas change.
- Run the relevant frontend, Rust, shell-contract, build, audit, and application checks before claiming completion.

## Publication controls

Do not change license assertions, copyright ownership, Bundle Identifier, release version mapping, public repository owner, signing, notarization, tags, remotes, or release materials without explicit user approval. Do not claim Homebrew endorsement. Never commit complete test tokens, private ledgers, tester identities, recipient labels, private manifests, build products, or full private audit reports.
