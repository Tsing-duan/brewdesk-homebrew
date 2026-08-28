# Architecture

BrewDesk is a local Tauri 2 application with a React/Vite frontend and a Rust backend. Two Swift helpers provide macOS-specific translation setup and execution. There is no cloud service or account tier in this source candidate.

## Components

1. **React frontend** — renders search, package detail, operation history, command previews, network status, and recovery actions. `src/brewApi.ts` is the typed bridge to Tauri commands and supplies browser-preview mocks only when the Tauri runtime is absent.
2. **Rust/Tauri backend** — discovers Homebrew, reads Homebrew metadata, validates targets, constructs argv, performs bounded network probes, classifies application conflicts, and serializes mutations through a single worker queue.
3. **Native helpers** — compiled from `src-tauri/swift` into an external build root and referenced by the packaged application. Their final bundled identity and assets remain release gates.
4. **Local data** — Homebrew command output and bounded caches stay on the user's machine. This candidate has no telemetry or inventory-upload path.

## Operation flow

```text
UI request
  -> typed Tauri command
  -> validate action and package token
  -> build executable + argv
  -> choose task-level network environment
  -> enqueue mutation
  -> single worker starts child process
  -> structured events and sanitized output return to UI
```

Read-only searches may use Homebrew metadata concurrently. State-changing Homebrew operations are queued. Cancellation distinguishes a queued operation from an active child process.

## Trust boundaries

Homebrew metadata, Cask artifacts, filesystem paths, network responses, and child-process output are not trusted merely because they are local. Application-origin detection is an evidence-based warning layer, not a universal installer provenance system. See `docs/SECURITY_MODEL.md`.

## Build boundaries

Repository files are source inputs only. Native helpers, Vite output, Cargo targets, Tauri bundles, and `.app` products belong in a private build root outside the repository. Final application verification reads the built `Info.plist` and Mach-O/code-signing structure; it does not require Gatekeeper or notarization success.
