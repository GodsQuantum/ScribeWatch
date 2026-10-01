# Changelog

All notable ScribeWatch changes are documented here.

## [0.5.1] - 2026-10-01

### Changed
- Completes French and Simplified Chinese coverage across provider, workflow, AI-profile, path-picker and advanced Live Recorder copy.
- Localizes built-in structure-profile names while preserving their stable backend IDs.

## [0.5.0] - 2026-10-01

### Added
- Persistent multilingual UI selector with English, French and Simplified Chinese.
- Browser microphone diagnostics for secure-context, permission and visible-device state, plus hot-plug refresh and stale-device fallback.
- Direct MD, TXT, HTML, DOCX, ODT and PDF export controls in Quick Transcribe and Live Recorder results.
- Separate built-in documentation profiles for general-medicine consultations and dental consultations, with explicit source attribution and an Items to verify section.

### Changed
- Live Recorder now requests microphone access from an explicit user action when permission is not already granted, watches device changes, applies only browser-supported speech constraints, and reduces MediaRecorder event overhead with larger chunks.
- Speech capture falls back to the current default input if a previously selected device disappears.
- Runtime base is upgraded to Alpine 3.24 (including FFmpeg 8.1.x), while the PDF engine is pinned to WeasyPrint 70.0 over Alpine's package baseline because 70.0 is the current security release.

### Safety
- Medical and dental prompts remain documentation-only: they must not infer diagnoses, treatments, findings, medication, measurements, tooth numbers/surfaces or consent that are absent from the transcript.
- Dental tooth notation is preserved as spoken; ScribeWatch does not silently convert between FDI, Universal, Palmer or other systems.
- Ambiguous clinically material values are surfaced for clinician verification rather than silently repaired.

## [0.4.0] - 2026-09-19

### Added
- Live Recorder with browser microphone discovery, device selection, timer, level meter and record/stop transcription.
- Content-based audio ingest with FFprobe plus FFmpeg normalization to mono 16 kHz PCM before STT; Watch folders are no longer limited by filename extensions.
- Optional OpenAI-compatible LLM providers kept separate from STT providers.
- Reusable AI structure profiles with built-ins for general notes, meetings/phone calls, medical-consultation documentation drafts, marketing brainstorms and interviews/research.
- Long-transcript AI structuring with bounded evidence chunks followed by a final synthesis pass.
- Document export from completed jobs to Markdown, plain text, HTML, DOCX, ODT and PDF.
- AI structuring state, model/profile metadata and non-fatal structuring errors in job history.
- CI verification for the arm64 container build in addition to the amd64 runtime smoke test.

### Changed
- The canonical transcript is now explicitly preserved when AI structuring is enabled; structured notes are an additional view in the same Markdown document.
- Quick Transcribe and Workflows can optionally select an AI structure profile without making an LLM mandatory.
- The runtime image now uses an Alpine base with a static musl Rust binary, retaining FFmpeg/Pandoc/WeasyPrint while keeping the image substantially smaller than the initial Debian export-runtime design.
- Temporary normalized audio and derived export working files live under the ephemeral data area and are cleaned after processing.

### Safety
- LLM prompts treat transcript content as untrusted data and explicitly forbid following instructions embedded in transcripts or inventing names, dates, measurements, diagnoses, commitments or other missing facts.
- AI structuring is best-effort: an LLM outage or invalid response never invalidates a successful transcript or blocks Markdown publication.
- The medical-consultation profile is documentation-only and forbids inferring diagnoses, examination findings, medications, doses, results or treatment plans not present in the transcript.
- Derived document exports neutralize Markdown image targets and disable raw HTML before Pandoc conversion.
- Leaving the Live Recorder view while recording stops capture without uploading an unfinished recording.

## [0.3.0] - 2026-09-19

### Added
- Optional deterministic transcript paragraphing using Unicode UAX #29 sentence boundaries.
- Fixed-size word-block fallback for punctuation-free STT transcripts without rewriting content.
- Quick Transcribe and Workflow controls for readable deterministic paragraphs, enabled by default.
- Ordered provider + model fallback chains for Workflows and Quick Transcribe, including multiple models from the same provider.
- Live model discovery per provider, per-route optional fallback timeouts, and durable per-route attempt history.
- Jobs now record the provider/model that actually succeeded and expose fallback attempts in the UI.

### Changed
- Transcription processing no longer inherits a provider-wide fixed request timeout; long audio waits by default unless a route explicitly defines a fallback timeout.
- Legacy `providerId + model` workflow and Quick Transcribe payloads remain accepted for backward compatibility.

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
