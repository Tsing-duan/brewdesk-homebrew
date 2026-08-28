# BrewDesk v0.1.0-alpha.1 Release Notes — Draft

Status: publication draft only. No tag or Release exists, and binary distribution is not approved.

## Summary

BrewDesk is an unofficial, Chinese-first Homebrew desktop client for Apple Silicon Macs. This first public Alpha source candidate focuses on visible command execution, conservative application-conflict checks, serialized Homebrew mutations, and task-scoped network routing that does not rewrite global Homebrew or shell configuration.

## Source candidate highlights

- Search Formulae and Casks with English names, Chinese names, and curated local aliases.
- Preview Homebrew commands before mutation operations.
- Queue install, uninstall, upgrade, and repair mutations through one worker.
- Protect existing applications when available path and receipt evidence indicate a source conflict.
- Apply proxy, direct, or mirror settings only to the current child process.
- Keep build products, audit artifacts, and private distribution material outside the public source tree.

## Version mapping

- Application version and `CFBundleShortVersionString`: `0.1.0`
- `CFBundleVersion`: `1`
- Planned prerelease tag: `v0.1.0-alpha.1`

The approved Bundle Identifier is `io.github.tsing-duan.brewdesk`. Final version-mapping evidence is recorded by Stage A against the built application's `Info.plist`; it does not constitute binary-distribution approval.

## Verification status

- Frontend tests: 18 passed, 0 failed.
- Rust tests: 34 passed, 0 failed, 0 ignored.
- Source web build, native helper builds, and the final ad-hoc-signed `.app` build: completed outside the repository.
- Built application mapping: `io.github.tsing-duan.brewdesk`, version `0.1.0`, bundle version `1`, minimum macOS `26.0`, arm64 main executable and two arm64 helpers.
- npm dependency advisories: none reported.
- RustSec known vulnerabilities: none reported; separately classified maintenance and target-specific warnings remain documented for manual review.

## Not included or approved

- No signed, notarized, or publicly distributable application binary.
- No Intel support claim and no macOS 14 or 15 support claim.
- No Homebrew endorsement, partnership, or official status.
- No publication approval until the final Git history scan, candidate review, and explicit Stage B approval are complete.

See `docs/KNOWN_LIMITATIONS.md`, `docs/AUDIT_SUMMARY.md`, `LICENSE`, and `NOTICE.md` before publication.
