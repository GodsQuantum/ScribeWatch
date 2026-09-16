# Changelog

All notable ScribeWatch changes are documented here.

## [Unreleased]

### Added
- Ordered provider + model fallback chains for Workflows and Quick Transcribe, including multiple models from the same provider.
- Live model discovery per provider, per-route optional fallback timeouts, and durable per-route attempt history.
- Jobs now record the provider/model that actually succeeded and expose fallback attempts in the UI.

### Changed
- Transcription processing no longer inherits a provider-wide fixed request timeout; long audio waits by default unless a route explicitly defines a fallback timeout.
- Legacy `providerId + model` workflow and Quick Transcribe payloads remain accepted during the v0.2.x migration window.

## [0.2.0] - 2026-09-16

### Added
- Quick Transcribe for browser uploads and allowed server-side audio files.
- Independent Watch, Markdown and Archive folders for automated workflows.
- Obsidian-friendly YAML properties, workflow tags and transcript-derived note titles.
- Client-side Markdown save via File System Access API with download fallback.
- New ScribeWatch visual identity, product-focused Home/Quick UI and real README screenshots.

### Changed
- Markdown filenames now derive from transcript titles and use collision-safe numbering.
- Shared Rust pipeline now serves both watched workflows and one-off transcription jobs.
- Container runtime now separates durable `/config` from bounded Quick Transcribe `/data`.

### Safety
- Quick Transcribe never archives or deletes the original source.
- Browser uploads are streamed with a configurable hard size limit and cleaned after completion/failure.
- Server source/destination paths remain constrained to canonical allowed roots.

## [0.1.0] - 2026-09-15

### Added
- Focused watch-folder pipeline: stable audio → OpenAI-compatible transcription → Markdown → archive.
- Rust/Axum provider, workflow, job/history, browse, health/readiness and SSE APIs.
- SvelteKit dashboard for providers, workflows, jobs, retries and current status.
- SQLite persistence, restart recovery, persistent deduplication and explicit schema migration.
- Native filesystem watching backed by periodic reconciliation for network filesystems.
- Secret-redacted provider API, allowed-root path validation and private SQLite permissions.
- Multi-stage non-root Docker image and hardened Compose example.

### Safety guarantees
- Source audio is never archived before Markdown publication succeeds.
- Existing Markdown and archive destinations are never silently overwritten.
- Failed transcription/publication keeps source audio in place.
- Retry after Markdown publication resumes archival without retranscribing.

### Removed from the AutoSubs fork
- Video rendering, subtitle editing, presets, brands, fonts, upload portal, assets and FFmpeg dependency.

Historical AutoSubs release details remain available in Git history before the ScribeWatch fork point.
