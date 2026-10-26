# Update reliability and validation

Updated: 2026-10-26. These changes retain the public Alpha identity, source-only distribution boundary, single mutation queue, and local data model.

## Changes

- Automatic routing checks final HTTP status and the Git smart-HTTP endpoint. A mirror returning 403 is no longer marked healthy merely because curl exited successfully. The normal GHCR 401 challenge is handled separately.
- A bundled Git helper restores task-local URL rules after Homebrew filters the startup environment. Existing mirror origins can be routed to the selected endpoint without writing global or repository Git configuration.
- Plain item counters such as `3/3` are not interpreted as byte totals. Transfer state is cleared between files and phases.
- A successful mutation process is followed by read-only checks of installation records and declared Cask application targets, including their main executable. These checks do not prove that the application will launch correctly or retain macOS privacy grants.
- Failed and completed operations invalidate stale inventory, outdated-package, and detail caches. Installed Cask versions support array-form metadata.
- Search, inventory, and details display available content before optional local translation finishes. Older requests cannot overwrite newer results. Refresh retains the current list; navigation and direct search save and restore each list's scroll position.
- Logs are merged in batches and rendered independently of progress-only updates. Reading older logs pauses automatic following and exposes a return-to-latest control.
- Application metadata is parsed once per bundle rather than starting a process for each field. XML and binary plists retain fallback handling for missing and damaged metadata.

The public source still contains no pre-generated curated Chinese description table, private inventories, tester records, or private runtime screenshots.

## Current source verification

- Node.js 24 and Rust 1.98.0; exact dependencies from the lockfiles.
- 32 frontend tests and 43 Rust tests pass without skipped or ignored cases. The public-only asset, command-injection, task-local environment, application-origin, and queue regressions remain included.
- TypeScript/web build, shell and CI contracts, external Rust-output boundary, Rust formatting, and Clippy checks pass.
- Local public application build verifies the approved Bundle Identifier, macOS 26 minimum, arm64 executables, ad-hoc signature structure, and executable Git helper. This is source-build evidence, not notarization or permission to distribute a binary.
- npm audit reports zero vulnerabilities. Targeted fixes use [Vitest 4.1.12](https://github.com/advisories/GHSA-qxg6-89mg-rqv8), [Vite 7.3.7](https://github.com/advisories/GHSA-587h-7wmj-h5pm), and [source-map-js 1.2.2](https://github.com/advisories/GHSA-68fv-2mgg-jv7q).
- cargo-audit 0.22.2 with RustSec database revision `21d909e4c8bd4e14c7aea58bf980c881a0053ab1` reports zero entries classified as vulnerabilities. It separately reports six unmaintained dependencies and two unsoundness warnings (`glib` and `tokio`); this is not an advisory-free dependency graph. No Linux support is claimed.

The original HTTP regression uses a local HTTP server and the real curl executable, covering success, denial, server errors, authentication scope, and redirect destinations. Removing the HTTP-status check makes the denial regression fail. Git regressions verify argument boundaries, filtered-environment behavior, and unchanged repository configuration.

## Runtime evidence and limits

The separately maintained local runtime completed a real in-app `brew update` on 2026-10-21 through the official route, and the installed Homebrew version was verified as 7.0.10. That observation does not establish current Homebrew freshness or substitute for testing this public build on another machine. Private runtime evidence is retained locally rather than published.

Bulk `brew upgrade`, Gatekeeper acceptance, app launch behavior after every Cask upgrade, historical privacy-permission prompts, and subjective overall smoothness remain outside this verification. Network probes cannot guarantee every third-party download host. No tag, public binary, expanded platform claim, or notarized release is included in this source update.
