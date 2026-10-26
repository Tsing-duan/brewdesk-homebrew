# Network Routing

BrewDesk offers automatic, system-proxy, direct, and mirror-oriented routes for individual Homebrew tasks. The selected route affects only environment variables passed to the child process that performs that task.

## Invariants

- No mode changes macOS Network settings.
- No mode writes Homebrew, Git, npm, Cargo, or shell-profile configuration.
- Direct mode removes inherited upper- and lower-case proxy variables from the child environment.
- Proxy mode supplies the detected proxy URL only to that child.
- Mirror mode supplies bounded Homebrew API and bottle variables only to that child. Git URL rules are applied through the bundled task-specific Git helper.
- Credentials must not be embedded in recorded proxy URLs, logs, screenshots, or issue reports.

Automatic selection uses bounded endpoint probes and local system evidence. A successful probe does not guarantee that every third-party Cask download host is reachable. A failed probe is diagnostic evidence, not permission to rewrite global configuration.

Probes follow redirects and inspect the final HTTP status. A curl exit code of zero alone is insufficient: 403, 404, and server errors are rejected. Only the GHCR registry entry point accepts its expected 401 authentication challenge. Git checks use the `info/refs?service=git-upload-pack` endpoint rather than a repository webpage. Loopback proxy discovery also recognizes Shadowrocket's `MacPacketTunnel` process; LAN listeners are excluded and every candidate is tested before selection.

Homebrew filters `GIT_CONFIG_*` variables before dispatch. BrewDesk selects its bundled helper through `HOMEBREW_GIT_PATH` and passes a fixed route selector in `HOMEBREW_BREWDESK_GIT_ROUTE`. The helper restores process-local URL rules immediately before invoking the system Git executable. This also overrides an existing mirror origin for the current task without rewriting the repository's origin or global Git configuration. Ordinary operations do not set `HOMEBREW_BREW_GIT_REMOTE`, which Homebrew may persist to origin.

Homebrew installation is a special interactive flow delegated to Terminal. Its displayed shell is constructed from fixed statements and shell-quoted detected URLs; BrewDesk does not collect the user's sudo password.
