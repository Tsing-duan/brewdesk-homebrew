# November 2026 maintenance

## Dependency scope

This maintenance updates React and React DOM to 19.3.1, their type packages to
19.3.0, the Tauri JavaScript API to 2.12.4, Rust Tauri to 2.12.5, plist to 1.10.3,
libc to 0.2.191, serde_json to 1.0.154, and Tokio to 1.53.3. The lockfiles record
required transitive changes. TypeScript, Vite, Vitest, the React build plugin,
and lucide major-version migrations are deferred. The exact local Tauri CLI
remains 2.11.4 and Rust remains 1.98.0.

Tokio 1.53.3 addresses [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html).
Upstream release references: [React](https://github.com/react/react/releases/tag/v19.3.1),
[Tauri](https://github.com/tauri-apps/tauri/releases/tag/tauri-v2.12.5),
[plist](https://github.com/ebarnard/rust-plist/releases/tag/v1.10.3),
[libc](https://github.com/rust-lang/libc/releases/tag/0.2.191),
[serde_json](https://github.com/serde-rs/json/releases/tag/v1.0.154).

The Rust toolchain and CI now request `llvm-tools-preview` explicitly. A minimal
local Rust installation otherwise emitted a missing `libLLVM.dylib` warning
when `rust-objcopy` tried to strip the release build.

## Interaction changes

1. **Search and navigation:** compact navigation retains visible text and explicit
   accessible names; the search submit button retains its name when its text is
   hidden. Keyboard focus uses a solid outline.
2. **Package details:** a compact window shows one usable panel at a time, with
   an explicit return action. The existing list stays mounted so results and
   scroll position survive. Returning invalidates pending detail requests and
   restores focus to the list region. Wide windows retain the two-column layout.
3. **Settings:** status and download routes come first. Optional translation and
   advanced Homebrew path settings use native disclosure controls; an unhealthy
   environment expands the path section automatically. The header refresh now
   actually rechecks the environment. The compact layout no longer reserves a
   second grid row when no detail panel exists.

The disclosure design follows [NN/G's progressive-disclosure guidance](https://www.nngroup.com/articles/progressive-disclosure/).
F-shaped scanning is an observed reading behavior, not a pass/fail layout rule;
[NN/G recommends clear headings and scannable content](https://www.nngroup.com/articles/f-shaped-pattern-reading-web-content/).
Visible targets and spacing were checked with [WCAG target-size guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
as a reference, not as a claim of complete WCAG conformance.

## Evidence and limits

The browser audit covers search, package details/return, and settings, using
preview data at 1053, 1440, and 390 CSS-pixel viewport widths. Compact desktop
navigation targets measured 59 by 58 CSS pixels. Before/after screenshots remain
in private local evidence; preview screenshots are not native runtime evidence.

Focused regression tests cover accessible navigation names, returning without
losing search results or scroll, and automatic disclosure after an environment
failure. Existing Homebrew route, argv, cancellation, progress parsing, and
installation-result verification tests remain part of the required checks.

This is a source maintenance update. It does not establish that real Homebrew
upgrades, historical privacy prompts, Gatekeeper, VoiceOver, all contrast states,
all dialogs, or every application flow have passed device acceptance. No installed
application is replaced and no public binary is distributed by this change.

Local checks pass: 35 frontend tests, 43 Rust tests, Rust formatting and Clippy,
CI/shell contracts, the external Rust-output boundary, web build, final arm64
application structure/ad-hoc signature checks, and secret scans. npm
audit reports zero vulnerabilities. RustSec audit reports zero vulnerabilities
and two informational warnings: unmaintained `proc-macro-error` 1.0.4
(RUSTSEC-2024-0370) and unsound `glib` 0.18.5 (RUSTSEC-2024-0429). These warnings
remain disclosed; zero vulnerabilities does not mean an advisory-free graph.
