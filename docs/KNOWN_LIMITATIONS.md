# Known Limitations

- Verified Stage A platform evidence is limited to Apple Silicon (`arm64`) on macOS 26. Intel and macOS 14/15 are not claimed.
- The interface and local-description experience are primarily Chinese-first.
- Application-origin classification cannot reliably identify every DMG, PKG, manual copy, backup restore, or management-system deployment.
- Network probes cover selected endpoints; a third-party Cask host may fail independently.
- Browser-preview mock data is not evidence of native Homebrew execution.
- A public screenshot is accepted only after capture from the final build and privacy review; screenshots are not runtime-validation evidence.
- The source license, application identifier, public repository identity, Git author identity, and generated icon authorization are approved for this candidate.
- Stage A uses ad-hoc signing only for structural verification. It does not prove Gatekeeper acceptance or notarization.
- Binary distribution remains unapproved pending a release-version dependency notice bundle, Developer ID signing, notarization, and explicit distribution approval.
- No signed public binary, Intel build, macOS 14/15 build, production readiness, or Homebrew endorsement is claimed.
