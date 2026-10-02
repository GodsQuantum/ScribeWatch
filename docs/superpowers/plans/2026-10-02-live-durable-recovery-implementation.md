# Durable LIVE Recording & Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ScribeWatch LIVE safe for multi-hour important recordings by checkpointing browser audio durably, recovering after crashes, supporting Continue/playback/final-audio retention, and preserving retryable server sources.

**Architecture:** Browser capture is split into durable OPFS-backed logical sessions containing one or more MediaRecorder segments. Cloud9 receives the completed segments through a dedicated streaming multipart endpoint, assembles one canonical M4A, optionally publishes a permanent server copy, persists a Quick/Live job, then transcribes from the durable source.

**Tech Stack:** Svelte 5, TypeScript/Node 24 tests, MediaRecorder, OPFS, IndexedDB/File System Access API, Rust 1.98.1, Axum multipart, Tokio, FFmpeg/ffprobe, SQLite/rusqlite, Docker.

**Spec:** `docs/superpowers/specs/2026-10-02-live-durable-recovery-design.md`

## Global Constraints

- Cloud9 only; canonical repo is `/srv/storage/cloud/arezki/Projets/Ssker Content Production/ScribeWatch`.
- No WebRTC transport, continuous server-side live streaming, live STT, tus protocol, new frontend framework, or new database.
- Do not change Cloud9 mounts, NFS architecture, allowed-root layout, or existing user storage paths.
- MediaRecorder checkpoint target remains approximately 5 seconds at 64 kb/s mono-oriented speech capture.
- Browser-delivered audio must never exist only in JS heap after a successful checkpoint.
- Default transient Quick/LIVE source retention is 24 hours.
- Final LIVE audio is one mono AAC/M4A produced by Cloud9; never byte-concatenate independent recorder segments.
- Keep existing Markdown publication, Workflow archive, recursive watch, and export behavior unchanged.
- Execution worktree, if used, must live inside the canonical repo at `.worktrees/live-durable-recovery`.
- Production deployment uses the next image tag `scribewatch:cloud9-20261002-r6` and retains `r5` as the sole immediate rollback.
- Rust tests run inside LXC 300 in an ephemeral Rust 1.98.1 container with FFmpeg installed; do not install Rust/FFmpeg on the Cloud9 Proxmox host.
## Review Focus

1. **OPFS quota/write failure mid-recording:** prior checkpoints must survive and the session must become recoverable instead of silently continuing without durability. Covered in Task 5.
2. **Duplicate/replayed LIVE upload after a lost HTTP 202:** the same `sessionId` must return the already accepted job rather than create another job or permanent audio copy. Covered in Task 4.
3. **Malformed/out-of-order/duplicate segment metadata:** server must reject it before finalization and never reorder based on multipart arrival. Covered in Task 3 and Task 4.
4. **Two start/continue actions while a recorder is already active:** controller must reject the second ownership attempt and preserve the active session. Covered in Task 6.
5. **Untrusted filenames/paths in LIVE metadata:** session ids, server destinations, and download paths must be validated; user input must never select a path outside configured allowed roots or LIVE source storage. Covered in Task 3 and Task 4.

---

### Task 1: Make Quick upload sources genuinely retryable

**Files:**
- Modify: `src/config.rs`
- Modify: `src/state.rs`
- Modify: `src/quick.rs`
- Modify: `src/pipeline_quick.rs`
- Modify/Test: `src/quick/tests.rs`

**Interfaces:**
- Produces config field `quick_source_retention_hours: u64`, env `SCRIBEWATCH_QUICK_SOURCE_RETENTION_HOURS`, default `24`.
- Produces cleanup rule based on job reference + `updated_at_ms`; Task 4 extends the same rule from Upload to Live sources.
- Existing `quick::cleanup_stale(&AppState) -> Result<CleanupStats>` remains the public cleanup entry point.
- [ ] **Step 1: Replace the old failure-deletes-source expectation with failing retention tests**

Add tests named:
- `failed_uploaded_quick_job_retains_source_for_retry`
- `restart_interrupted_quick_job_keeps_source_during_cleanup`
- `expired_terminal_upload_becomes_cleanup_eligible`
- `deleted_upload_job_becomes_cleanup_eligible`

Assertions: failed/interrupted sources exist; retry succeeds while retained; an unreferenced/expired source is removed by cleanup.

- [ ] **Step 2: Run the targeted tests and verify RED**

Run in the Rust test container:
`cargo test --locked quick::tests::failed_uploaded_quick_job_retains_source_for_retry -- --nocapture`

Expected: failure because `process_quick_job` currently deletes uploaded sources unconditionally.

- [ ] **Step 3: Add the dedicated 24-hour source-retention config**

Add `quick_source_retention_hours` to `Config`, defaulting to 24, and update every test Config literal.

- [ ] **Step 4: Remove unconditional uploaded-source deletion from `process_quick_job`**

The pipeline must leave transient uploaded sources in place after success, error, cancellation, or interruption. Cleanup becomes the only expiry mechanism.

- [ ] **Step 5: Make cleanup reference-aware**

Keep Upload sources while the owning job is active or `now_ms - job.updated_at_ms <= retention`. Jobs absent from state or beyond retention do not protect their file.

- [ ] **Step 6: Refresh restart timestamp**

When AppState converts an active persisted job to `Interrupted`, also refresh `updated_at_ms` so a restart grants a fresh retention window.

- [ ] **Step 7: Run targeted + Quick tests and verify GREEN**

Run: `cargo test --locked quick::tests -- --nocapture`
Expected: all Quick tests pass.

- [ ] **Step 8: Commit**

`git add src/config.rs src/state.rs src/quick.rs src/pipeline_quick.rs src/quick/tests.rs && git commit -m "Preserve Quick audio for reliable retry"`
### Task 2: Enforce transcription provider timeout

**Files:**
- Modify: `src/provider.rs`
- Modify/Test: `src/transcription_chain.rs`

**Interfaces:**
- Existing `provider::transcribe(...)` signature is unchanged.
- `Provider.timeout_seconds` becomes an actual total HTTP request deadline.
- Route `fallback_after_seconds` remains an independent outer deadline.

- [ ] **Step 1: Add failing timeout tests**

Add:
- `provider_timeout_fails_a_hung_route`: one provider sleeps longer than its 1-second timeout and the job records a failed attempt.
- Extend/retain `route_timeout_falls_through`: provider timeout is longer than route fallback, so fallback still wins and the next route succeeds.

- [ ] **Step 2: Run and verify RED**

Run: `cargo test --locked transcription_chain::tests::provider_timeout_fails_a_hung_route -- --nocapture`
Expected: timeout test exceeds the provider deadline or does not finish as expected.

- [ ] **Step 3: Apply the request timeout**

In `provider::transcribe`, call RequestBuilder `.timeout(Duration::from_secs(provider.timeout_seconds.max(1)))` before `.send()`.

- [ ] **Step 4: Run timeout/fallback tests and verify GREEN**

Run: `cargo test --locked transcription_chain::tests -- --nocapture`
Expected: provider timeout is bounded; route fallback semantics remain green.

- [ ] **Step 5: Commit**

`git add src/provider.rs src/transcription_chain.rs && git commit -m "Enforce STT provider request timeouts"`
### Task 3: Add LIVE domain model and deterministic finalization core

**Files:**
- Create: `src/live.rs`
- Modify: `src/lib.rs`
- Modify: `src/domain.rs`
- Modify: `src/config.rs`
- Test: unit tests inside `src/live.rs`

**Interfaces:**
- In `src/domain.rs`, add `QuickSourceKind::Live` and `LiveJobMeta { session_id: String, started_at_ms: u128, audio_result_name: String, interruption_gaps_ms: Vec<u64> }`; add optional `live: Option<LiveJobMeta>` to `QuickJobMeta`.
- In `src/live.rs`, add `LiveAudioRetention::{None, Server { directory: String }, Client}`.
- In `src/live.rs`, add `LiveUploadSegment { id: u32, mime_type: String, started_at_ms: u128, ended_at_ms: Option<u128>, gap_ms_before: Option<u64> }`.
- In `src/live.rs`, add `LiveUploadManifest { schema_version: u32, session_id: String, started_at_ms: u128, segments: Vec<LiveUploadSegment>, retention: LiveAudioRetention, options: QuickOptions }` with `#[serde(rename_all="camelCase")]`.
- Serialize `LiveAudioRetention` as an internally tagged union with `#[serde(tag="mode", rename_all="snake_case")]`, yielding `{mode:"none"}`, `{mode:"server",directory:"..."}`, or `{mode:"client"}`.
- Add `Config::live_staging_dir() -> PathBuf` and `Config::live_source_dir() -> PathBuf`.
**Interfaces continued:**
- `live::validate_manifest(&LiveUploadManifest) -> Result<Uuid>` validates schema/session/segment metadata only.
- `live::resolve_server_destination(&Config, &LiveAudioRetention) -> Result<Option<PathBuf>>` enforces allowed-root path policy.
- `live::segment_paths_in_manifest_order(&LiveUploadManifest, &HashMap<u32, PathBuf>) -> Result<Vec<PathBuf>>`.
- `live::finalize_segments(&Config, &LiveUploadManifest, &HashMap<u32, PathBuf>, &CancellationToken) -> Result<FinalizedLiveAudio>`.
- `FinalizedLiveAudio { source_path: PathBuf, audio_name: String, server_copy: Option<PathBuf> }`.
- Canonical source path: `/data/live-sources/<session-id>.m4a`.
- AAC target: mono AAC-LC, 64 kb/s; preserve recorded order, do not insert fake silence.

- [ ] **Step 1: Add failing validation/order tests**

Tests:
- `manifest_requires_schema_v1_and_uuid_session`
- `manifest_rejects_duplicate_or_non_contiguous_segment_ids`
- `manifest_order_controls_segment_order_not_hashmap_order`
- `server_retention_path_must_resolve_inside_allowed_roots`

- [ ] **Step 2: Run validation tests and verify RED**

Run: `cargo test --locked live::tests::manifest_ -- --nocapture`
Expected: module/types/functions absent.

- [ ] **Step 3: Implement domain types and manifest validation**

Reject empty segment sets, ids other than contiguous 1..N, invalid timestamps, invalid UUID session ids, or Server retention directories outside allowed roots.

- [ ] **Step 4: Add failing FFmpeg finalization test**

Create two tiny WAV fixtures, deliberately provide their map in reverse insertion order, finalize, and assert the result exists and `audio::probe_audio` returns true. Assert final filename is deterministic from session/start metadata.

- [ ] **Step 5: Implement finalization**

Generate an FFmpeg concat-list from validated manifest order, decode/concat/re-encode to a staging M4A, `sync_all()`, then atomically install `<session-id>.m4a` in `live_source_dir`. Do not read segment bytes into Rust memory.

- [ ] **Step 6: Implement deterministic server-copy publication**

For Server retention, copy into a temp file in the chosen destination, flush+`sync_all()`, and publish without overwrite. The permanent filename includes start timestamp plus an 8-char session-id suffix so retries are deterministic.

- [ ] **Step 7: Run `live` tests and verify GREEN**

Run: `cargo test --locked live::tests -- --nocapture`
Expected: all finalization/validation tests pass with ffmpeg installed in the test container.

- [ ] **Step 8: Commit**

`git add src/live.rs src/lib.rs src/domain.rs src/config.rs && git commit -m "Add durable LIVE audio finalization core"`
### Task 4: Add streaming LIVE ingest, idempotent acceptance, and audio download

**Files:**
- Create: `src/api/live.rs`
- Modify: `src/api/mod.rs`
- Modify: `src/quick.rs`
- Modify: `src/api/jobs.rs`
- Modify/Test: `src/quick/tests.rs`
- Test: unit/integration tests inside `src/api/live.rs`

**Interfaces:**
- New route: `POST /api/v1/live/upload`, body limit disabled at Axum layer; multipart total is enforced by app config.
- Multipart field 1 must be `manifest` JSON; following fields are `segment-0001`, `segment-0002`, etc.
- `quick::find_live_job_by_session(&AppState, &str) -> Option<Job>`.
- `quick::create_live_job(&AppState, PathBuf, String, LiveJobMeta, QuickOptions) -> Result<Job>`.
- New route: `GET /api/v1/jobs/{id}/audio` for retained Live sources only.
- [ ] **Step 1: Add failing multipart/idempotency tests**

Tests:
- `live_upload_streams_segments_and_returns_accepted_job`
- `live_upload_rejects_segment_before_manifest`
- `live_upload_rejects_duplicate_unknown_or_missing_segment_part`
- `live_upload_rejects_invalid_audio_segment`
- `replayed_session_id_returns_existing_job_without_duplicate_copy`
- `replayed_accepted_session_with_missing_source_returns_conflict`

- [ ] **Step 2: Run and verify RED**

Run: `cargo test --locked api::live::tests -- --nocapture`
Expected: route/module absent.

- [ ] **Step 3: Implement streaming staging**

Require the first `manifest` JSON field to be at most 64 KiB. Create `live-staging/<upload-uuid>/`; stream each media field chunk directly to a new file; count total bytes against `max_upload_bytes`; flush+`sync_all()` every completed segment. Never call `.bytes()` on media fields.

- [ ] **Step 4: Implement session-id idempotency**

Immediately after validating the manifest, if a job with matching `quick.live.session_id` exists and its source file exists, return that job with HTTP 202 without creating another final source or permanent copy. If the job exists but its retained source is missing, return HTTP 409 instead of creating a second job under the same session id.

- [ ] **Step 5: Validate segments, finalize, persist job, then enqueue**

Probe every received segment, call Task 3 finalization, ensure requested Server copy has succeeded, persist the Live Quick job, enqueue it, then return HTTP 202. Only accepted jobs may be exposed as successful.

- [ ] **Step 6: Extend cleanup to LIVE sources and stale staging**

`quick::cleanup_stale` applies Task 1 retention rules to `QuickSourceKind::Live`, cleans orphan `live-sources` files after retention, and removes stale `live-staging` directories older than the same 24-hour window.

- [ ] **Step 7: Add guarded audio download tests**

Tests:
- `live_job_audio_download_returns_retained_m4a`
- `audio_download_rejects_non_live_job`
- `audio_download_rejects_source_outside_live_source_dir`

- [ ] **Step 8: Implement `GET /api/v1/jobs/{id}/audio`**

Canonicalize both configured LIVE source root and job source. Require `QuickSourceKind::Live`, `quick.live.is_some()`, source inside root, and existing file. Return attachment filename from `audio_result_name`.

- [ ] **Step 9: Run API/Quick tests and verify GREEN**

Run:
- `cargo test --locked api::live::tests -- --nocapture`
- `cargo test --locked quick::tests -- --nocapture`
- `cargo test --locked api::jobs::tests -- --nocapture`
Expected: all LIVE API, Quick lifecycle, and audio-download tests pass.

- [ ] **Step 10: Commit**

`git add src/api/live.rs src/api/mod.rs src/api/jobs.rs src/quick.rs src/quick/tests.rs && git commit -m "Add durable LIVE upload and download APIs"`
### Task 5: Implement durable browser recovery storage

**Files:**
- Create: `frontend/src/lib/live-recovery.ts`
- Create/Test: `frontend/tests/live-recovery.test.mjs`

**Interfaces:**
- `LiveSessionState = 'new'|'recording'|'interrupted'|'finalizing'|'upload_pending'|'uploading'|'accepted'|'completed'`.
- `LiveRetention = {mode:'none'} | {mode:'server';directory:string} | {mode:'client';handleKey:string}`.
- `LiveSegmentManifest { id, mimeType, startedAtMs, endedAtMs?, lastCheckpointAtMs?, chunkCount, bytes, gapMsBefore? }`.
- `LiveSessionManifest { schemaVersion:1, id, state, createdAtMs, startedAtMs, updatedAtMs, lastCheckpointAtMs?, mimeType, audioBitsPerSecond:64000, segments, totalBytes, options:QuickOptions, retention:LiveRetention, acceptedJobId? }`.
- `LiveRecoveryStore.createSession(manifest: LiveSessionManifest): Promise<void>`.
- `LiveRecoveryStore.getSession(id: string): Promise<LiveSessionManifest|undefined>` and `listRecoverable(): Promise<LiveSessionManifest[]>`.
- `LiveRecoveryStore.appendChunk(sessionId: string, segmentId: number, blob: Blob, checkpointAtMs: number): Promise<LiveSessionManifest>`.
- `LiveRecoveryStore.updateSession(manifest: LiveSessionManifest): Promise<void>` and `buildSegmentBlob(sessionId: string, segmentId: number): Promise<Blob>`.
- `LiveRecoveryStore.deleteSession(id: string): Promise<void>`.
- `LiveRecoveryStore.saveClientDirectoryHandle(sessionId: string, handle: FileSystemDirectoryHandle): Promise<void>` and `loadClientDirectoryHandle(sessionId: string): Promise<FileSystemDirectoryHandle|undefined>`.
- `OpfsLiveRecoveryStore` is the production implementation; tests inject an in-memory fake implementing the same interface.
- [ ] **Step 1: Add failing store tests**

Tests:
- `append_chunk_updates_manifest_only_after_durable_write`
- `recording_manifest_is_returned_as_interrupted_after_recovery_scan`
- `segment_blob_preserves_chunk_order`
- `delete_session_removes_manifest_chunks_and_saved_handle`
- `write_failure_keeps_previous_checkpoint_metadata_unchanged`

- [ ] **Step 2: Run and verify RED**

Run: `cd frontend && npm test`
Expected: live-recovery module/tests fail before implementation.

- [ ] **Step 3: Implement OPFS manifest/chunk store**

Use `navigator.storage.getDirectory()`; create `scribewatch-live/v1/<session-id>/segments/<4-digit-id>/`. Each chunk write creates/closes its own writable before manifest replacement.

- [ ] **Step 4: Implement manifest replacement discipline**

Write new manifest content only after the chunk file has closed successfully. A crash before the manifest update leaves an unreferenced chunk that recovery can safely ignore; never advance durable metadata before data exists.

- [ ] **Step 5: Implement persistence/quota helpers**

Export `requestPersistentStorage(): Promise<boolean|undefined>` and `storageEstimate(): Promise<{usage?:number;quota?:number}>`. Unsupported APIs return `undefined`, not an exception.

- [ ] **Step 6: Implement IndexedDB directory-handle storage**

Store client `FileSystemDirectoryHandle` under session id; load/delete through the `LiveRecoveryStore` interface. Permission is always rechecked before writing.

- [ ] **Step 7: Run recovery tests and verify GREEN**

Run: `cd frontend && npm test`
Expected: existing tests plus new recovery tests all pass.

- [ ] **Step 8: Commit**

`git add frontend/src/lib/live-recovery.ts frontend/tests/live-recovery.test.mjs && git commit -m "Add durable browser LIVE recovery storage"`
### Task 6: Move recorder ownership into a persistent LIVE controller

**Files:**
- Create: `frontend/src/lib/live-recorder.ts`
- Create/Test: `frontend/tests/live-recorder.test.mjs`
- Modify: `frontend/src/routes/+page.svelte`

**Interfaces:**
- `LiveStartConfig = { options: QuickOptions; retention: LiveRetention; mimeType: string; audioBitsPerSecond: 64000 }`.
- `LiveRecorderSnapshot = { activeSessionId?: string; recording: boolean; elapsedMs: number; protectedBytes: number; stream?: MediaStream; error?: string }`.
- `LiveRecorderController.startSession(config: LiveStartConfig, stream: MediaStream): Promise<LiveSessionManifest>`.
- `LiveRecorderController.continueSession(sessionId: string, stream: MediaStream): Promise<LiveSessionManifest>`.
- `LiveRecorderController.stop(): Promise<LiveSessionManifest>`.
- `LiveRecorderController.requestCheckpoint(): void`.
- `LiveRecorderController.interrupt(reason: string): Promise<void>`.
- `LiveRecorderController.subscribe(listener: (snapshot: LiveRecorderSnapshot)=>void): ()=>void`.
- `getLiveRecorderController(): LiveRecorderController` returns one browser-lifetime singleton.
- The controller owns MediaRecorder, MediaStream, checkpoint write queue, timer timestamps, visibility listener, and Wake Lock; views only subscribe.
- [ ] **Step 1: Add failing controller tests**

Tests:
- `checkpoint_is_persisted_before_protected_bytes_advance`
- `continue_creates_next_segment_and_records_gap_without_silence`
- `storage_write_failure_interrupts_and_preserves_prior_chunks`
- `recorder_error_interrupts_recoverably`
- `track_ended_interrupts_recoverably`
- `second_start_or_continue_is_rejected_while_active`
- `destroying_a_view_subscription_does_not_stop_recording`
- `hidden_page_requests_checkpoint`
- `elapsed_uses_timestamps_not_interval_tick_count`

- [ ] **Step 2: Run and verify RED**

Run: `cd frontend && npm test`
Expected: new live-recorder tests fail before implementation.

- [ ] **Step 3: Implement start/checkpoint/stop state machine**

Use `MediaRecorder.start(5000)`; queue every non-empty `dataavailable` through `store.appendChunk`. Do not retain successfully persisted chunk Blobs in a long-lived array.

- [ ] **Step 4: Implement Continue**

Load Interrupted session, compute `gapMsBefore = max(0, newSegmentStart - previous.lastCheckpointAtMs)`, append the next segment id, and start a new MediaRecorder without modifying old segment bytes.

- [ ] **Step 5: Implement incident handling**

On recorder error, track ended, or checkpoint write failure, stop capture where possible, drain already queued successful writes, mark Interrupted, and keep the recovery session.

- [ ] **Step 6: Implement visibility/Wake Lock lifecycle**

On hidden visibility call `requestData()` when recorder is active. Acquire Wake Lock when supported and reacquire after visibility restoration; Wake Lock failure is warning-only.

- [ ] **Step 7: Make route navigation preserve singleton ownership**

`+page.svelte` may unmount LiveRecordView, but must never recreate/stop the singleton controller. No recorder lifecycle remains in component destruction.

- [ ] **Step 8: Run controller + frontend tests and verify GREEN**

Run: `cd frontend && npm test && npm run check`

- [ ] **Step 9: Commit**

`git add frontend/src/lib/live-recorder.ts frontend/tests/live-recorder.test.mjs frontend/src/routes/+page.svelte && git commit -m "Make LIVE recording survive navigation and interruption"`
### Task 7: Integrate recovery, playback, retention choices, and final upload UI

**Files:**
- Modify: `frontend/src/lib/types.ts`
- Modify: `frontend/src/lib/api.ts`
- Modify: `frontend/src/lib/local-save.ts`
- Modify: `frontend/src/lib/views/LiveRecordView.svelte`
- Modify: `frontend/src/lib/i18n.ts`
- Modify: `frontend/src/app.css`
- Modify/Test: `frontend/tests/local-save.test.mjs`
- Modify/Test: `frontend/tests/scribewatch-surface.test.mjs`
- Create/Test: `frontend/tests/live-finalize.test.mjs`

**Interfaces:**
- Frontend mirrors Task 3 Live types and `QuickSourceKind = 'server'|'upload'|'live'`.
- `api.liveUpload(manifest: LiveUploadManifest, segments: Array<{id:number;blob:Blob}>): Promise<Job>`.
- `api.jobAudio(id: string): Promise<Response>`.
- `saveResponseToDirectory(response: Response, filename: string, handle?: FileSystemDirectoryHandle): Promise<'directory'|'download'>`.
- `seekLastSeconds(audio: HTMLAudioElement, seconds = 30): void`.
- [ ] **Step 1: Add failing integration/helper tests**

Tests:
- `live_upload_form_puts_manifest_before_numbered_segments`
- `failed_live_upload_keeps_recovery_session`
- `accepted_none_or_server_session_is_deleted_after_202`
- `client_save_failure_keeps_recovery_session`
- `seek_last_30_seconds_clamps_to_zero`
- `unsupported_directory_picker_uses_download_fallback`
- Surface test asserts LiveRecordView no longer has `chunks:Blob[]` or `onDestroy()->recorder.stop()`.

- [ ] **Step 2: Run and verify RED**

Run: `cd frontend && npm test`
Expected: new integration assertions fail.

- [ ] **Step 3: Add LIVE API/client types**

FormData order is manifest first, then numbered fields `segment-0001`, `segment-0002`, etc. `jobAudio` returns the raw Response so supported browsers can stream the body directly to a file handle.

- [ ] **Step 4: Add streaming client save**

When a writable directory handle exists and permission is granted, create the target file and stream `response.body` into its writable stream; do not materialize the full M4A Blob. Unsupported picker or missing stream uses existing download fallback.

- [ ] **Step 5: Refactor LiveRecordView into controller UI**

Remove recorder/stream/chunks ownership. Subscribe to Task 6 controller, keep microphone selection/acquisition UI, and attach the audio meter to the controller-owned active stream while the view is mounted.

- [ ] **Step 6: Add recovery card and playback**

On LIVE mount, list recoverable sessions. Default playback to the newest completed segment using `store.buildSegmentBlob`, expose native scrub controls, segment selector when needed, and “Listen to last 30 seconds”.

- [ ] **Step 7: Add Continue / Finish and transcribe / Save audio / Delete recovery**

Continue acquires a current microphone and calls controller Continue. Finish uploads all segment Blobs in manifest order. “Save audio” selects Client retention then executes the same server finalization/transcription path; it does not invent a second finalizer. Delete requires explicit confirmation and is disabled for the currently active session.

- [ ] **Step 8: Add final-audio retention selector**

Options: None, Server folder, This computer. Server uses existing PathPicker. Client asks for a directory before recording when supported and remembers its handle; unsupported browsers clearly state that completion will use a download.

- [ ] **Step 9: Apply cleanup-after-success rules**

After 202: None/Server may delete OPFS recovery; Client deletes only after finalized audio save/download succeeds. Any upload/client-save failure leaves recovery intact and exposes Retry.

- [ ] **Step 10: Add compact durability diagnostics**

Show durable/protected bytes, persistent-storage status, microphone status, and elapsed duration. Keep internal chunk terminology out of the normal path.

- [ ] **Step 11: Run frontend gates and verify GREEN**

Run: `cd frontend && npm test && npm run check && npm run build`
Expected: all tests pass, 0 Svelte errors/warnings.

- [ ] **Step 12: Commit**

`git add frontend/src/lib/types.ts frontend/src/lib/api.ts frontend/src/lib/local-save.ts frontend/src/lib/views/LiveRecordView.svelte frontend/src/lib/i18n.ts frontend/src/app.css frontend/tests && git commit -m "Add recoverable long-duration LIVE recording UX"`
### Task 8: Full regression, documentation, deployment, and cleanup

**Files:**
- Modify: `README.md`
- Modify: `README.fr.md`
- Modify deployment only: `/srv/lxc/travail/compose/scribewatch/compose.yaml`
- No product logic changes unless verification exposes a defect; any defect returns to the owning task's RED/GREEN cycle first.

**Interfaces:**
- Compose adds `SCRIBEWATCH_QUICK_SOURCE_RETENTION_HOURS: '24'`.
- Production image: `scribewatch:cloud9-20261002-r6`.
- Rollback image retained: `scribewatch:cloud9-20261002-r5`.

- [ ] **Step 1: Run complete Rust regression**

In the Rust/ffmpeg test environment:
`cargo test --locked --all-targets`
Expected: all Rust tests pass.

- [ ] **Step 2: Run complete frontend regression**

`cd frontend && npm test && npm run check && npm run build`
Expected: all Node tests pass; Svelte check has 0 errors/0 warnings; build succeeds.

- [ ] **Step 3: Build production image**

Build `scribewatch:cloud9-20261002-r6` from the exact reviewed Git tree. Dockerfile build must rerun its own frontend check/tests/build and Rust release compile.

- [ ] **Step 4: Perform synthetic crash/reload smoke test**

Record at least 20 seconds, verify protected bytes grow, reload, verify recovery appears and is playable, seek to the end/last 30 seconds, Continue another segment, Stop, and finalize.

- [ ] **Step 5: Verify server-retained audio**

Finalize with a temporary allowed server folder. Confirm exactly one playable M4A exists, source job is Live, and the permanent file survives STT failure/retry tests. Remove only the synthetic test artifacts afterward.

- [ ] **Step 6: Verify network/upload failure path**

Force the LIVE endpoint unavailable for a synthetic session; confirm OPFS recovery remains and retry works after restoring service.

- [ ] **Step 7: Verify ScribeWatch restart path**

After accepted upload but before/while STT completes, restart only the ScribeWatch container. Confirm job becomes Interrupted, retained source exists, and Retry processes the same source.

- [ ] **Step 8: Update README documentation**

Document long-duration durability, recovery/Continue semantics, interruption gaps, retention choices, 24-hour transient source window, and the fact that this is resilience functionality rather than medical certification/compliance.

- [ ] **Step 9: Commit documentation**

`git add README.md README.fr.md && git commit -m "Document durable LIVE recording recovery"`

- [ ] **Step 10: Deploy r6**

Update only the ScribeWatch image tag and retention env in its compose file, run `docker compose up -d scribewatch`, wait for healthy, and verify `/api/v1/ready` returns version 0.5.2.

- [ ] **Step 11: Verify production UI/API**

Confirm LIVE page includes recovery/retention UI, normal short recording still works, workflows remain recursive/archive-aware, and no unrelated service/container changed.

- [ ] **Step 12: Push/synchronize GitHub**

Use MCPproxy GitHub write tooling if direct git transport remains unavailable. Verify remote files/tree match the deployed reviewed source before claiming sync.

- [ ] **Step 13: Final cleanup**

Remove test containers, `target/`, `frontend/node_modules`, `.svelte-kit`, `frontend/build`, synthetic audio/recovery/staging artifacts, and dangling images. Keep only r6 active plus r5 rollback. Final git working tree must be clean.

- [ ] **Step 14: Final evidence report**

Report test counts, active image/health, Git SHA/tree, recovery smoke outcomes, retained-source/retry outcomes, and cleanup inventory.
