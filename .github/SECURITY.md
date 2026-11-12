# Security policy

## Supported versions

Security fixes target the latest released ScribeWatch version and `main`. Older versions may need to upgrade before receiving a fix.

## Reporting a vulnerability

Do not open a public issue containing vulnerability details. Use the GitHub Security advisory / private vulnerability reporting flow when enabled, or contact the maintainer privately. Never include credentials, patient details or exploit instructions in public issues.

## Deployment security

ScribeWatch can read and write explicitly mounted media and can send audio to administrator-configured transcription endpoints. Treat it as an administrative service.

- Do not expose it directly to the public Internet; use an authenticated reverse proxy or VPN.
- Mount only required watch/archive paths and keep `SCRIBEWATCH_ALLOWED_ROOTS` narrow.
- Keep `/config` on local storage. SQLite may contain provider API keys; ScribeWatch forces the DB file to mode `0600` on Unix.
- The browser never receives stored API-key values, only `hasApiKey`.
- Container examples run non-root, drop capabilities, use `no-new-privileges`, and keep the root filesystem read-only.
