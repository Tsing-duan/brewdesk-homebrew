# Source Release Process

The public repository is canonical. The first source prerelease,
`v0.1.0-alpha.1`, was published on 2026-08-28. Early Stage A documents describe
that initial export and its historical gates; their no-remote/no-tag rules are
not requirements to erase the now-published repository.

## Maintenance prereleases

1. Obtain explicit approval for the source release and its scope. Approval to
   update source does not authorize publishing an application binary.
2. Keep private inventories, ledgers, full audit logs, builds, and temporary
   tools outside the repository. Preserve existing license and identity decisions.
3. Verify the implementation with the pinned toolchains, relevant tests, audits,
   secret scans, output boundaries, and native application checks. Evidence may
   be reused for unchanged implementation; identify its exact commit and limits.
4. Refresh both READMEs, verification records, release notes, screenshot provenance,
   and links. Capture native screenshots using `screenshots/CAPTURE_GUIDE.md`.
5. Check the exact staged public tree, local links, image metadata, secret-scan
   Canary, and complete Git history. Push without rewriting published history.
6. Confirm the release commit and its CI status. Create a new tag at that exact
   commit, retain older tags/releases, and publish a GitHub prerelease with source
   archives only. Do not mark an Alpha as a stable Latest release.
7. Verify the published tag target, release text, prerelease flag, and absence of
   binary assets. Save the publication receipt outside the repository.

## Version mapping

- Tauri/macOS application version: `0.1.0`
- macOS `CFBundleVersion`: `1`
- First source prerelease: `v0.1.0-alpha.1`
- November 2026 source maintenance prerelease: `v0.1.0-alpha.2`

Source prerelease tags identify source snapshots and do not change the app's
numeric version mapping. Never feed prerelease SemVer into
`CFBundleShortVersionString`. Inspect the built `Info.plist` for verification.

## Binary distribution

Source-release approval does not approve binary distribution. Developer ID
signing, notarization, stapling, redistribution obligations, asset rights, and
supported-platform evidence remain separate. Ad-hoc signature verification is
not Gatekeeper acceptance or Apple notarization.
