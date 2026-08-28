# Asset Sources

This document covers assets distributed as part of the BrewDesk source release. Dependency licenses remain governed by their upstream projects and are summarized separately in `NOTICE.md`.

## Application icon

The BrewDesk application icon was generated specifically for this project with OpenAI Codex at the user's direction. It was produced in two stages: Codex first generated a BrewDesk-specific barrel-and-mug draft, then used only that generated draft as the reference for the final liquid-glass variant.

No user-supplied or third-party image, logo, icon library, template, bundled font, webpage image, or copied SVG/path data was used. The icon does not include a Homebrew, Apple, GitHub, or OpenAI logo. BrewDesk does not claim that the generated design is unique, does not claim exclusive trademark rights, and does not claim endorsement by OpenAI or Homebrew.

The retained raster master is:

`design/brewdesk-icon-liquid-glass.png`

It is a flattened 1254×1254 RGB PNG. No layered or vector source exists. The master remains editable with ordinary raster tools, while the exact generated pixels are identified by SHA-256.

The complete platform icon set was produced from that one master with the project-local Tauri CLI:

```text
npm run tauri -- icon design/brewdesk-icon-liquid-glass.png
```

The Stage A generation used Tauri CLI 2.11.4. Hashes for the master and every generated output are recorded in `docs/ICON_ASSET_SHA256.txt`.

## Fonts

BrewDesk distributes no custom font file. CSS font-family declarations select system or locally installed fonts; those fonts are not copied or redistributed by this repository.

## Package metadata and descriptions

Package names, descriptions, versions, homepages, and related metadata are read locally from Homebrew or its installed metadata. BrewDesk does not represent those fields as an official BrewDesk dataset.

The source release contains no pre-generated curated Chinese description table. Existing Chinese upstream descriptions may be displayed as supplied. Optional English-to-Chinese description translation uses the user's local macOS translation capability and cache. Project-maintained Chinese search aliases are navigation terms and are not described as official Homebrew Chinese data.

## Screenshots

Repository screenshots must be captured from the final rights-cleared BrewDesk build and privacy-reviewed before publication. They must not expose user paths, private software inventory, tester or recipient information, private commands, credentials, or unlicensed third-party artwork. Each retained screenshot is documented alongside its capture conditions.

`docs/screenshots/brewdesk-search.png` was captured on 2026-08-28 from the verified Stage A `BrewDesk.app` on macOS 26.6.2 (`arm64`) after the approved icon was applied to the in-app brand area. It shows the neutral initial search screen, contains no package inventory or operation history, was converted to metadata-free PNG for inclusion, and has SHA-256 `33cc157469dbc6ff31f938742a36b38ca60018d19c5649b37cffe73a939a7fe1`.
