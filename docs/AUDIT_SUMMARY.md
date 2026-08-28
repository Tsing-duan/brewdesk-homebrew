# Public Audit Summary

This is the redacted Stage A summary for the local BrewDesk public-release candidate. The complete report, command evidence, absolute paths, dependency inventories, tool artifacts, and audit deviation record are stored outside this repository with private `0700` directory and `0600` file permissions.

## Source

- The candidate was created by a sanitized allowlist export with `lstat`, realpath containment, special-file, symbolic-link, hard-link, case-collision, identity, and source/target SHA-256 checks.
- The original source remained read-only. Private distribution material, generated dependencies, caches, build outputs, archives, and native binaries were not imported into the public tree.
- The known private ledger was never intentionally opened, hashed, copied, archived, or supplied to an approved content scan. Its path state, type, owner, permissions, and exclusion state were checked privately.
- One exploratory text-search invocation had ineffective exclusions and was stopped immediately. Its output was not used for any conclusion, no ledger match/path/value appeared, no output file was retained, and nothing from it was written to the candidate or transmitted. The private report records this as an audit deviation rather than treating it as positive scan evidence.
- Unverified pre-generated Chinese description text was removed. Existing upstream Chinese descriptions and optional local macOS translation remain available without being represented as official Homebrew Chinese data.

## Security

- Gitleaks `8.30.0` was used; `8.30.1` was not used. The official archive checksum, archive SHA-256, binary SHA-256, and active configuration SHA-256 were verified.
- Before each formal scan batch, a dynamically generated hit Canary matched the dedicated rule and returned exit code `1`; the no-hit control returned `0`. Byte-level checks confirmed the complete Canary token was absent from stdout, stderr, reports, command traces, and CI-log inputs.
- The final public worktree scan and the complete local Git history scan reported zero findings.
- `npm audit` reported zero info, low, moderate, high, or critical vulnerabilities. `nanoid` is locked at `3.3.18` and `postcss` at `8.5.24`; the specified advisories are absent.
- `cargo-audit 0.22.2` reported zero known vulnerabilities against the recorded RustSec database revision. Sixteen unmaintained-package warnings and one GTK-related unsound advisory remain disclosed for manual dependency review; they were not hidden or reclassified as vulnerabilities resolved by BrewDesk.

## Verification

- Canonical Stage A baselines: frontend `16`, Rust `30`.
- Final unfiltered results: frontend `18 passed`, Rust `34 passed`; zero failed, skipped, ignored, todo, measured, or filtered tests.
- Coverage includes mutation-token injection rejection, executable-plus-argv construction, task-scoped network variables, prohibition on global configuration writes, application-conflict protection, queue behavior, public asset boundaries, and the approved in-app brand asset.
- `cargo fmt --check` and `cargo clippy --locked --all-targets --all-features -- -D warnings` passed with Rust `1.98.0`.
- Node.js `24.20.0`, npm `11.19.0`, and the project-local exact `@tauri-apps/cli 2.11.4` were used. No global Tauri CLI was required.
- The final `.app` was built outside the repository and structurally ad-hoc signed. `app:verify` confirmed `CFBundleIdentifier=io.github.tsing-duan.brewdesk`, `CFBundleShortVersionString=0.1.0`, `CFBundleVersion=1`, minimum macOS `26.0`, and arm64 architecture for the main executable and both helpers.
- Gatekeeper acceptance, notarization, stapling, Developer ID identity, and public binary distribution were intentionally not required or claimed.

## Documentation

- MIT `LICENSE`, source/binary license classification, `NOTICE.md`, asset provenance, governance, contribution, security, support, architecture, release-process, and agent instructions are present.
- The retained BrewDesk icon was generated specifically for BrewDesk with Codex from a BrewDesk-specific generated draft; no user-supplied or third-party image, logo, icon library, template, bundled font, webpage image, or copied SVG/path data was used. The user authorized redistribution without claiming uniqueness, exclusive trademark rights, or endorsement.
- The editable raster master and the complete Tauri icon set are retained with SHA-256 records and a reproducible project-local Tauri generation command.
- The public screenshot was captured from the final verified `.app`, inspected at full size, and contains no user path, software inventory, private operation history, credential, tester/recipient data, private manifest, notification, or unrelated application.

## Publication

- Local branch: `main`.
- Git author identity was derived from the objectively verified GitHub login and numeric account ID and stored only in repository-local configuration.
- Every workflow `uses:` reference is pinned to an approved full 40-character commit SHA. Checkout credential persistence is disabled; the secret-scan job fetches complete history; setup-node uses Node.js 24 with package-manager cache disabled.
- Stage A has no remote, no tag, no push, and no Release. The intended repository URL is `https://github.com/Tsing-duan/brewdesk-homebrew`, but it is not configured locally.
- The public repository becomes canonical only after an explicitly approved release.

## Status fields

```text
NAME_COLLISION_ACKNOWLEDGED=true
LICENSE_TARGET=MIT
LICENSE_STATUS=SOURCE_RELEASE_CLEARED_BINARY_DISTRIBUTION_BLOCKED_PENDING_NOTICE_BUNDLE
GITLEAKS_CANARY_PASS=true
GITLEAKS_OUTPUT_REDACTION_VERIFIED=true
GIT_IDENTITY_VERIFIED=true
APP_VERSION_MAPPING_VERIFIED=true
PRIVATE_AUDIT_ARTIFACTS_OUTSIDE_PUBLIC_REPO=true
PUBLIC_REPOSITORY_BECOMES_CANONICAL_AFTER_RELEASE=true
RUST_TOOLCHAIN_PIN_VERIFIED=true
CARGO_AUDIT_VERSION_VERIFIED=true
RUSTSEC_DB_COMMIT_RECORDED=true
ALL_GITHUB_ACTIONS_SHA_PINNED=true
NO_REMOTE_CONFIGURED=true
NO_TAGS_PRESENT=true
GIT_WORKTREE_CLEAN=true
```

## BLOCKERS

- `SOURCE_RELEASE_BLOCKER`: none identified for this candidate.
- `BINARY_DISTRIBUTION_BLOCKER`: public binary distribution is not approved. A release-version third-party license/NOTICE bundle, MPL-2.0 covered-source availability record, Developer ID signing, notarization, and explicit binary-distribution approval remain required.
- `MANUAL_LICENSE_REVIEW_REQUIRED`: repeat the dependency and notice review for the exact release commit, target, features, and packaged binary contents.

## APPROVAL_REQUIRED

Stage A created only a local source-release candidate. Stage B requires separate explicit approval before configuring a remote, pushing, creating `v0.1.0-alpha.1`, creating a GitHub Release, uploading any artifact, or contacting a third party.
