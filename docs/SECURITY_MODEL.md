# Security Model

## Protected outcomes

BrewDesk aims to prevent command injection, unintended global configuration changes, concurrent Homebrew mutations, silent replacement of applications installed from another source, accidental secret publication, and misleading source-build claims.

## Command boundary

Backend operations select a known executable and pass arguments as an array. Mutation targets must satisfy the package-token grammar before they reach Homebrew. Search accepts broader Unicode text because it is not used as a mutation command token. Shell string concatenation is not an acceptable substitute for argv construction.

## Network boundary

Proxy and mirror variables are applied to the current child process only. Direct mode clears inherited proxy variables for that child. BrewDesk does not persist those settings globally. Details and limitations are in `NETWORK_ROUTING.md`.

## Filesystem and application-origin boundary

For Casks, BrewDesk inspects declared application targets and available local evidence, including managed Homebrew paths and Mac App Store receipts. An existing application from an unconfirmed source causes a conservative conflict warning. This cannot prove provenance for every DMG, PKG, manual copy, restore, or device-management installation.

## Process and queue boundary

State-changing operations pass through one mutation queue. Cancellation must distinguish waiting work from an active child process. UI previews are explanatory; the backend validator and argv builder remain authoritative.

## Secret and privacy boundary

The public tree must not contain private ledgers, tester identities, recipient labels, private manifests, credentials, or complete Canary tokens. Full Stage A audit artifacts remain outside the repository with private permissions; only a redacted summary is public.

## Explicit non-goals

- No password or sudo-secret collection.
- No persistence of system proxy or Homebrew mirror configuration.
- No telemetry, behavioral analytics, account service, or software-inventory upload.
- No claim that ad-hoc signing is Gatekeeper acceptance, notarization, or approval for binary distribution.
- No universal guarantee about third-party Cask hosts or application provenance.

## Release verification

Security gates include dynamic Gitleaks hit/no-hit Canaries, secret scans, npm and Rust advisory checks, dependency-license classification, focused regression tests, external-output enforcement, built `Info.plist` validation, Mach-O architecture checks, and ad-hoc code-signing structure verification.
