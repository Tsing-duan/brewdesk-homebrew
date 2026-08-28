# Network Routing

BrewDesk offers automatic, system-proxy, direct, and mirror-oriented routes for individual Homebrew tasks. The selected route affects only environment variables passed to the child process that performs that task.

## Invariants

- No mode changes macOS Network settings.
- No mode writes Homebrew, Git, npm, Cargo, or shell-profile configuration.
- Direct mode removes inherited upper- and lower-case proxy variables from the child environment.
- Proxy mode supplies the detected proxy URL only to that child.
- Mirror mode supplies bounded Homebrew API, bottle, and repository variables only to that child.
- Credentials must not be embedded in recorded proxy URLs, logs, screenshots, or issue reports.

Automatic selection uses bounded endpoint probes and local system evidence. A successful probe does not guarantee that every third-party Cask download host is reachable. A failed probe is diagnostic evidence, not permission to rewrite global configuration.

Homebrew installation is a special interactive flow delegated to Terminal. Its displayed shell is constructed from fixed statements and shell-quoted detected URLs; BrewDesk does not collect the user's sudo password.
