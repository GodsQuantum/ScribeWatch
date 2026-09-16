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
