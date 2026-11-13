# ScribeWatch technical decisions

Research was performed on 2026-09-15 through the required private SearXNG instance. Raw JSON results are retained in `research/raw/`.

## Watcher: native notifications plus reconciliation

**Decision:** keep `notify` events as a low-latency accelerator, but treat a periodic directory scan as authoritative.

Network filesystems can miss, delay, coalesce, or implement filesystem events differently. A reconciliation pass also repairs missed events after application restarts. Candidates must pass ScribeWatch's size+mtime stability window before a job is created.

Sources:
- https://docs.rs/notify/
- https://github.com/notify-rs/notify

## Persistence and migrations

**Decision:** use SQLite on local storage only, WAL mode, FULL synchronous writes, and explicit `PRAGMA user_version` migrations.

The schema is deliberately small. `rusqlite_migration` was evaluated, but adding a migration dependency for one compact schema was not justified yet. The migration boundary remains explicit so adopting it later is straightforward.

Sources:
- https://github.com/cljoly/rusqlite_migration
- https://docs.rs/rusqlite_migration

## Transcription provider contract

**Decision:** providers configure a complete OpenAI-compatible audio transcription URL, model, optional bearer token, and timeout. Audio is streamed as multipart directly from disk.

Nothing is hardcoded to Speaches or a specific cloud. This keeps local Speaches, compatible gateways, and external APIs on one abstraction. Provider secrets stay server-side; API responses expose only `hasApiKey`.

Sources:
- https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create
- https://developers.openai.com/api/docs/guides/speech-to-text
- https://docs.vllm.ai/en/latest/serving/online_serving/speech_to_text/

## Markdown publication and archival

**Decision:** publish the Markdown peer before moving source audio. Publication uses a unique peer temporary file, flush + `sync_all`, then no-overwrite hard-link publication. A collision is a visible job failure and the source remains in place.

Archive moves reserve a unique destination. Same-filesystem hard links provide atomic no-overwrite moves; a create-new copy path covers cross-filesystem moves. Archive failure after Markdown publication is retryable without retranscribing.

Research background:
- https://users.rust-lang.org/t/how-to-write-replace-files-atomically/42821
- https://users.rust-lang.org/t/correct-way-to-save-a-file-atomically-but-without-interferring-with-performance/89223

## HTTP/API stack

**Decision:** retain Axum/Tokio from AutoSubs and expose a narrow API: health/readiness, dashboard, providers, workflows, jobs, browse, and SSE.

This keeps Rust in charge of validation and business invariants while the SvelteKit frontend remains a thin API client.

Sources:
- https://github.com/tokio-rs/axum

## SvelteKit frontend

**Decision:** keep the current Svelte 5 / SvelteKit 2 static frontend architecture, remove the subtitle/video UI, and enforce zero `svelte-check` warnings.

The app uses semantic labels, responsive navigation, and server-side path browsing rather than exposing arbitrary filesystem access.

Source:
- https://svelte.dev/docs/kit/accessibility

## Container packaging

**Decision:** use a multi-stage image. Node exists only in the frontend build stage, Rust tooling only in the builder, and runtime contains only the ScribeWatch binary, static UI, CA certificates, and curl for healthchecks.

The Compose example runs non-root, drops all Linux capabilities, enables `no-new-privileges`, uses a read-only root filesystem, and explicitly mounts config/watch/archive paths.

Sources:
- https://docs.docker.com/build/building/best-practices/
- https://docs.docker.com/build/building/multi-stage/
- https://docs.docker.com/reference/compose-file/services/

## Deliberately deferred

No LLM is required for v1. Summary, title, chapters, action items, speakers, and tags remain future enrichment hooks rather than hard dependencies. No video rendering, subtitle editing, asset library, presets, or manual upload workflow is retained from AutoSubs.

## Universal, profile-independent source-fidelity policy (2026-11-13)

**Decision:** data processing and accuracy safeguards belong to the ScribeWatch
**engine**, never to a fixed list of built-in profile IDs. Every current
and future user-created profile inherits:

- The unchanged source transcript as the canonical factual record; the profile
  prompt and optional template may control *structure and relevance*, never
  overwrite source evidence or silently invent missing fields.
- Whole-source coverage, including the first/middle/final passages and
  contradictions. A long source is split and processed in order, with
  context-aware passage sizes and bounded retries on actual context errors.
- On local embedded LLMs, an optional evidence ledger when the available
  context budget permits (source-only factual extraction followed by a
  source-verified synthesis); if not, source-preserving ordered sections
  remain available instead of a truncated or invented completion.
- A bounded omission/contradiction review of actual source passages on local
  models, for *any* profile ID, with exact-span quotation, unchanged numbers,
  speaker-claim checks and a conservative reject-on-uncertainty policy.
- Profile-aware language, terminology and preferred headings. Dental drafts
  retain their clinically tested specialized prompt; meetings preserve
  decisions versus proposals, brainstorms preserve minority/rejected ideas,
  medical drafts preserve uncertainty and attribution. Unknown custom profiles
  receive a safe source-grounding default rather than an empty policy.

**Hardware adaptation:** model loading, quantization, context and chunk sizes
depend on detected resources; the requirement to preserve every source part
does not. CPU-only, low-VRAM and large-VRAM paths share the same safety and
profile contracts. Performance and output quality still require measurement
on representative machines; a unit test or a passing installer does not
certify accuracy.

**Release gates:** the private, permission-controlled benchmark must compare
identical source-audio SHA-256, actual ASR output, the app's real structured
report, clinician-reviewed content and export PDFs. A 48-item lexical
checklist is a diagnostic, **not** a stand-alone clinical correctness metric.
Regression checks must also use synthetic long/short inputs across all
built-in profiles and randomly named custom profiles. Never publish patient
transcripts, names, PDFs, or results in public source-control histories.

**UTF-8 benchmark correctness:** QA tools must decode raw HTTP JSON response
**bytes** as strict UTF-8 before reading or scoring multilingual content.
Windows PowerShell 5.1 may silently decode Content-Type application/json
without an explicit charset as Latin-1; accent corruption changes lexical
scores, document pagination and hashes. Do not diagnose ASR or model
regressions from corrupted benchmark transcripts. Verify untouched source
SHA-256 and keep private references outside the public repository.

**Current qualification status (v0.6.5, 2026-11-13):** all release
platform workflows, native installer gates and public-repository privacy
checks passed. The native Windows end-to-end pipeline on two separate
workstations produced complete, source-grounded dental drafts and document
exports from the same private benchmark audio. The two Windows installations
and the self-hosted Docker deployment were updated successfully, preserving
existing configuration and job history. The original benchmark regression
claim was invalidated: Windows PowerShell 5.1 corrupted UTF-8 JSON during
benchmark collection, not during application transcription or writing.
Scores against a single private lexical checklist **do not establish
clinical accuracy, patient safety or quality on all hardware or future
custom profiles**. Custom profiles inherit the engine safeguards and have
automated regression tests, but representative human-reviewed evaluation
remains necessary. No patient media or private reference data belong in
this repository.
