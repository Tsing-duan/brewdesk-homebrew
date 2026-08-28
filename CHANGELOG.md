# Changelog

All notable public changes will be recorded here. The format follows Keep a Changelog conventions, and release identifiers use Semantic Versioning where applicable.

## Unreleased

### Added

- Public Alpha documentation, governance, security boundaries, and verification entry points.
- Repository-external native-helper and application-verification contracts.
- A privacy-reviewed screenshot captured from the verified Stage A application build.

### Changed

- Private distribution material is excluded from the sanitized allowlist export.
- User-facing beta wording is replaced with public Alpha wording.
- The public build contract uses Node.js 24 and the project-local exact Tauri CLI.
- The in-app brand area now uses the approved BrewDesk icon instead of a letter placeholder.
- Unverified pre-generated Chinese description text was removed; upstream Chinese text and optional local macOS translation remain supported.

### Security

- Stage A secret scanning uses checksum-verified Gitleaks 8.30.0 with a dynamic hit and no-hit Canary before formal scan batches.
- Source and binary distribution rights are classified separately: the source candidate is MIT-licensed, while public binary distribution remains unapproved.

No release has been published and no tag exists during Stage A.
