# Third-Party Notices

BrewDesk source code is licensed under the MIT License. Third-party dependencies, system frameworks, and upstream package metadata remain under their own terms and are not relicensed by BrewDesk.

## Direct JavaScript dependencies

| Component | Version in lockfile | License |
| --- | ---: | --- |
| `@tauri-apps/api` | 2.11.1 | Apache-2.0 OR MIT |
| `lucide-react` | 0.525.0 | ISC |
| `react` | 19.2.8 | MIT |
| `react-dom` | 19.2.8 | MIT |

Development dependencies and their complete transitive versions are recorded in `package-lock.json`. Build-only dependencies remain governed by their own licenses, including the CC-BY-4.0 data package `caniuse-lite`.

## Rust dependencies

The exact Rust dependency versions are recorded in `src-tauri/Cargo.lock`. The Apple Silicon macOS graph includes components offered under MIT, Apache-2.0, ISC, BSD, Zlib, Unicode-3.0, CC0, and Unlicense-family terms.

The graph also includes unmodified MPL-2.0 components such as `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`. Their covered source remains available from the repositories identified in their Cargo package metadata. BrewDesk does not relicense or claim ownership of those files.

## System software and upstream data

- Apple frameworks used by BrewDesk are provided by macOS and are not redistributed in this source repository.
- Homebrew names, metadata, formulae, casks, and related upstream material remain governed by their respective projects and rights holders.
- The BrewDesk application icon is project-specific generated material documented in `docs/ASSET_SOURCES.md`.

## Binary distribution status

This source candidate is not approval to distribute BrewDesk binaries. A public binary package must include a version-specific third-party license collection and any required upstream NOTICE material, including the selected terms for compound SPDX expressions and MPL-2.0 covered-source information. Developer ID signing, notarization, and explicit release approval are also outside this source-license notice.

The lockfiles are the authoritative version inventory. If a dependency or build target changes, regenerate and manually review the binary notice bundle before distribution.
