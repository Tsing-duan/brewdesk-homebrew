# Security Policy

## Supported scope

Security review currently covers the latest source on the public Alpha candidate branch. No signed, notarized, or production-supported binary is offered by this Stage A candidate.

High-value boundaries include package-token validation, executable-plus-argv command construction, task-scoped network variables, prohibition of global configuration writes, application-origin conflict protection, operation queuing, local helper execution, and Tauri capabilities.

## Reporting a vulnerability

Do not publish an exploit, token, private package inventory, private manifest, tester identity, recipient label, or unredacted machine path in an issue.

The final private reporting channel depends on the user-confirmed GitHub owner and repository settings. After the repository exists, use GitHub Private Vulnerability Reporting if it is enabled. If it is not available, open only a non-sensitive issue asking the maintainers to establish a private channel. Do not send secret material until the channel is verified.

Include the affected revision, platform, impact, minimal sanitized reproduction, and whether the report is believed to be actively exploitable. Maintainers will acknowledge and triage reports as project capacity permits; no response-time SLA is promised for this Alpha.

## Disclosure

Coordinate public disclosure with maintainers after a fix and verification evidence exist. Never use a security report to test changes against another person's machine, Homebrew installation, or account without explicit authorization.
