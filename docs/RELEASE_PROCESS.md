# Release Process

Stage A prepares a local public-release candidate. It does not configure a remote, create a tag, push, publish a Release, sign with Apple credentials, notarize, or contact a third party.

## Stage A gates

1. Produce a sanitized allowlist export while keeping the original source read-only.
2. Keep full audits and build products in separate private directories outside the repository.
3. Verify Gitleaks 8.30.0 checksums and run a dynamic hit/no-hit Canary before each formal scan batch.
4. Resolve dependency advisories and classify licenses as source blockers, binary-distribution blockers, or manual review requirements.
5. Verify documentation, version mapping, pinned toolchains, full-SHA GitHub Actions, tests, external builds, built `Info.plist`, architecture, and ad-hoc code-signing structure.
6. Only after every prerequisite, create a local Git history with a user-confirmed author name and exact GitHub noreply address.
7. Prove no remote, no tag, and a clean worktree. Generate release notes and publication commands as drafts only.
8. Obtain explicit approval before any Stage B publication action.

## Version mapping

- Tauri/macOS application version: `0.1.0`
- macOS `CFBundleVersion`: `1`
- Planned first public prerelease tag: `v0.1.0-alpha.1`

If `package.json` ever uses a prerelease SemVer, it must not feed `CFBundleShortVersionString`. The built application's `Info.plist`, rather than source configuration alone, is authoritative verification evidence.

## Binary distribution

Source-release approval does not approve binary distribution. Signing identity, notarization, stapling, redistribution obligations, asset rights, and supported-platform evidence remain separate. Stage A may verify an ad-hoc signature's structure without requiring Gatekeeper or notarization acceptance.
