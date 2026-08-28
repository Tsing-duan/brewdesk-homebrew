# Publication Checklist — Draft Only

This checklist prepares a possible Stage B. It does not authorize repository creation, remote configuration, push, tag creation, Release creation, binary upload, or third-party contact.

## Confirmed Stage A decisions

- Name-collision risk: acknowledged.
- MIT source-release authority: confirmed and retained in the private audit record.
- Approved copyright-holder wording: `Copyright (c) 2026 BrewDesk contributors`.
- The BrewDesk-specific Codex-generated icon is authorized and documented; no custom font files are distributed; uncertain pre-generated Chinese descriptions were removed; final screenshots require privacy review.
- Approved Bundle Identifier: `io.github.tsing-duan.brewdesk`.
- Git author: `Tsing-duan`; GitHub-provided noreply identity was verified privately through GitHub's user API.
- Intended repository: `https://github.com/Tsing-duan/brewdesk-homebrew`; no separate homepage.
- The public repository becomes canonical only after the approved release: confirmed.

## Stage A gates before any publication approval

- Keep the approved MIT `LICENSE`, public `NOTICE.md`, and asset-source record aligned with the release tree.
- Complete strict all-target Clippy, the repository-external `.app` build, and `app:verify` using the final identity and approved assets.
- Verify the built `Info.plist`, `arm64` architecture, minimum macOS version, executable/helper completeness, and ad-hoc code-signing structure. Do not require Gatekeeper, notarization, or stapling acceptance.
- Re-run Gitleaks 8.30.0 dynamic hit/no-hit Canary and output-redaction checks immediately before each formal scan batch.
- Initialize local Git only after every history prerequisite is satisfied; use repository-local Git identity only.
- After the final commit, scan the complete Git history and prove that all workflow Action references use approved full commit SHAs.
- Prove that `git remote -v`, `git tag --list`, and `git status --porcelain=v1 --untracked-files=all` produce no output.
- Produce the private full candidate report and retain only the redacted public audit summary in the repository.

## Explicit Stage B approval required

Only an explicit user approval after reviewing the candidate report may authorize publication actions. Stage A does not configure a remote, create a tag, push, or create a Release.
