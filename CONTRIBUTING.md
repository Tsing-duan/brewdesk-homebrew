# Contributing to BrewDesk

Thank you for helping improve BrewDesk. This repository is preparing an early public Alpha; small, reviewable changes with explicit evidence are preferred.

## Before opening a change

1. Read `AGENTS.md`, `docs/ARCHITECTURE.md`, and `docs/SECURITY_MODEL.md`.
2. Check `docs/KNOWN_LIMITATIONS.md` so that a known boundary is not presented as a regression.
3. For a bug, record the macOS version, CPU architecture, BrewDesk revision, expected behavior, observed behavior, and a sanitized reproduction.
4. Never attach access tokens, complete shell profiles, private package inventories, private distribution manifests, or unredacted absolute paths.

## Development contract

- Use Node.js 24 and `npm ci`.
- Use the exact Rust toolchain declared by `rust-toolchain.toml` once that file is finalized.
- Invoke the project-local Tauri CLI through `npm run tauri`; do not depend on a globally installed CLI.
- Direct every Rust, Swift, Vite, Tauri, and application artifact to an absolute `PUBLIC_BUILD_ROOT` outside the repository.
- Keep network routing task-scoped. Do not write proxy or mirror values to Homebrew, macOS, Git, npm, Cargo, or shell global configuration.
- Preserve the executable-plus-argv command boundary and strict mutation-token validation.

## Verification

Run the public verification entry points appropriate to the change. The final release gate requires at least the re-verified Stage A baselines: 16 passing frontend tests and 30 passing Rust tests. Counts may increase but may not decrease. Skipped, ignored, filtered, todo, or assertion-free cases do not count.

Changes to command construction, token validation, task-level network variables, global-write prohibitions, application conflict protection, or queue behavior require focused regression tests.

## Scope

Good Alpha contributions fix documented defects, improve tests or documentation, or make a narrowly justified security and reliability improvement. Large UI redesigns, cloud services, accounts, telemetry, AI features, and broad rewrites should not be introduced without prior maintainer agreement.

## License and release changes

BrewDesk source is MIT-licensed. Do not replace the license, change copyright assertions, publish a binary, create a release tag, or claim official Homebrew affiliation without explicit maintainer approval. New dependencies and assets must include reviewable provenance and license evidence.
