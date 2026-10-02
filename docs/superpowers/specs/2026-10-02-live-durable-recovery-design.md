# ScribeWatch durable LIVE recording and recovery design

Date: 2026-10-02
Status: Approved conversational design; written-spec review pending
Scope: Browser LIVE capture, crash recovery, durable audio retention, server retry semantics, STT timeout hardening

## Purpose

LIVE recording must be safe enough for long, important sessions such as a medical consultation.
No important recording may exist only in JavaScript heap memory.
A browser crash, reload, internal navigation, network failure, microphone loss, or ScribeWatch restart must preserve as much already-captured audio as the platform has delivered.
The design favors a small number of explicit durable states over streaming complexity.

This improves loss resilience; it does not by itself make ScribeWatch a certified medical-record system or establish regulatory compliance.

## Non-goals

- No WebRTC transport.
- No continuous server-side live streaming.
- No live speech-to-text requirement.
- No tus/resumable-upload protocol in this iteration.
- No new database or frontend framework.
- No changes to Cloud9 mounts, NFS architecture, or allowed-root layout.

## Core invariants

1. Browser-delivered LIVE audio is durably checkpointed to local browser storage during recording.
2. A successfully checkpointed chunk may be discarded from RAM.
3. Local recovery data is not deleted until Cloud9 durably accepts the final audio and any requested client-side copy is complete.
4. A server-side uploaded audio source remains retryable after transcription failure or process restart.
5. A server-side save destination is written before transcription starts.
6. An archive/save failure never masquerades as successful transcription completion.
7. Internal ScribeWatch navigation never terminates an active recording.
8. Recovery never fabricates missing audio or silently bridges a crash gap.
## Browser persistence model

Use the Origin Private File System (OPFS) as the durable recording store.
Request persistent storage with `navigator.storage.persist()` when available and expose storage usage/quota with `navigator.storage.estimate()`.
Failure to obtain persistent classification is a warning, not a blocker; failure to write a checkpoint is a blocking recording error.

Canonical OPFS layout:

```text
/scribewatch-live/v1/<session-id>/
  manifest.json
  segments/
    0001/
      chunk-000001.blob
      chunk-000002.blob
    0002/
      chunk-000001.blob
```

Each MediaRecorder `dataavailable` payload is written as a separate chunk file and closed before it is considered durable.
This deliberately favors simple crash semantics over a more complex worker-based append protocol.
At a five-second cadence, even multi-hour recordings create only a few thousand small files.

The manifest is versioned and contains:
- session id and schema version;
- lifecycle state;
- created/start/update timestamps;
- selected MIME type and target bitrate;
- segment list and per-segment timestamps;
- last durably stored chunk number;
- accumulated observed duration;
- chosen transcription/options snapshot;
- requested final-audio retention mode;
- optional server destination path;
- client destination reference key, never the directory handle itself;
- upload/job identifiers once Cloud9 accepts the recording.

The user's picked client directory handle is stored separately in IndexedDB because FileSystemDirectoryHandle is structured-cloneable while JSON is not.
## LIVE session lifecycle

Browser-visible states:

```text
new
 -> recording
 -> interrupted
 -> recording         (Continue creates a new segment)
 -> finalizing
 -> upload_pending
 -> uploading
 -> accepted
 -> completed
```

`recording` found during application startup is treated as `interrupted`; a browser that died cannot still own that recorder.

A user Stop:
1. requests any currently buffered recorder data;
2. stops the MediaRecorder;
3. waits for the final `dataavailable`;
4. waits for the OPFS write queue to drain;
5. closes the segment;
6. marks the session `finalizing`.

A recorder/microphone error follows the same durability path where possible, but ends in `interrupted`.

The elapsed timer is derived from timestamps/monotonic elapsed time, not by counting timer callbacks or assuming exact MediaRecorder timeslices.
## Continue after crash or reload

Opening LIVE scans OPFS for unfinished sessions.
Each recoverable session is presented before starting a new recording.

The recovery card shows:
- original start time;
- last durable checkpoint time;
- estimated saved duration and size;
- number of segments;
- estimated interruption gap where known;
- local playback controls for saved audio.

Actions:
- Continue;
- Finish and transcribe;
- Save audio;
- Delete recovery.

Continue never appends bytes to an old MediaRecorder container.
It starts a new MediaRecorder segment under the same logical session id.
The manifest records the wall-clock gap between the last durable checkpoint and the new segment start.
No artificial silence is inserted to hide that gap.

The UI defaults playback to the most recent completed segment because that is the audio immediately preceding the interruption.
If a session has multiple interruption segments, the user can select earlier segments.
Each segment has a normal audio seek bar, plus an “Listen to last 30 seconds” action that seeks to max(0, duration - 30s).

A recovery session can therefore be listened to and verified before Continue.
## Active recording ownership and navigation

MediaRecorder ownership moves out of LiveRecordView into a small client-side LIVE session controller.
The controller survives normal ScribeWatch view changes.
LiveRecordView becomes a subscriber/control surface rather than the owner of the capture object.

Navigating to Home, Jobs, Workflows, or another in-app view does not stop recording.
When the LIVE view is reopened it reattaches to the active controller state and, if available, rebuilds the visual level meter from the active stream.

Full page reload/crash still destroys the recorder; OPFS recovery handles that case.

During recording:
- request a Screen Wake Lock when supported;
- reacquire the lock after visibility restoration when appropriate;
- on `visibilitychange` to hidden, call `recorder.requestData()` if active;
- listen for `MediaRecorder.error`;
- listen for active microphone track `ended`;
- treat permission/device loss as an interrupted recoverable session.

`beforeunload` may provide a warning but is never a durability mechanism.
## Local audio retention choices

LIVE exposes one explicit final-audio choice:

```text
Keep final audio
  None
  Server folder
  This computer
```

### Server folder

The existing ScribeWatch path picker selects an allowed server directory.
Cloud9 creates a unique final `.m4a` there before transcription begins.
The operation uses existing allowed-root validation and must not alter mounts or NFS configuration.

Recommended filename:
`live-YYYY-MM-DD_HH-mm-ss.m4a`, with existing-style numeric collision handling.

### This computer

On supported secure browsers, `showDirectoryPicker({mode:'readwrite'})` selects the destination before recording.
The handle is stored in IndexedDB using the session id as the key.
At finalization ScribeWatch rechecks permission and asks again if required.
If directory picking is unsupported, the UI transparently falls back to a normal audio download.

The final client file is the server-finalized M4A, not a naïve concatenation of independently restarted browser containers.
If the local copy has not completed, OPFS recovery is retained.
## Server LIVE ingest

Add a dedicated LIVE upload endpoint instead of overloading the existing Quick multipart contract.

Request contains:
- one JSON/options field with session metadata and transcription options;
- one media part per completed browser segment.

The browser reconstructs each segment Blob from that segment's OPFS chunks.
It does not build the entire multi-hour recording in RAM.

Server flow:
1. create a private staging directory under persistent ScribeWatch data;
2. stream every multipart segment directly to a new staging file;
3. enforce the configured total upload-size limit;
4. flush and `sync_all()` each completed staged file;
5. validate that each segment contains decodable audio;
6. assemble/transcode all segments in chronological order with FFmpeg;
7. produce one mono AAC/M4A final source at a speech-appropriate bitrate;
8. `sync_all()` the final source;
9. if Server folder retention was requested, atomically copy/publish the final M4A there and sync it;
10. create and persist the Quick job referencing the durable final source;
11. only then return HTTP 202 and the job.

An interrupted HTTP upload leaves no accepted job.
The browser still owns the OPFS recovery and can retry the whole upload.
Old incomplete staging directories are cleanup candidates.
## Multi-segment finalization

Separate MediaRecorder instances can produce independent container headers and must not be byte-concatenated blindly.

Within one segment, chunks from the same MediaRecorder are reconstructed in sequence as the browser-generated segment Blob.
Across interruption segments, Cloud9 uses FFmpeg decode + concat + encode to produce one canonical M4A.

The server must preserve segment order from the validated manifest rather than multipart arrival order alone.
Manifest segment ids must be unique and contiguous for finalization.

An interruption gap is metadata only.
The final audio contains the recorded speech before and after the interruption without fabricated silence.
The job/audit metadata retains the gap estimate for UI display.

If final assembly fails, the staged uploaded segments remain available for retry within the server retention window.
## Quick/LIVE source retention and retry

The current unconditional deletion of uploaded Quick audio after pipeline execution is removed.

For all Quick Upload sources:
- Pending/active: always retain;
- Error: retain;
- Interrupted: retain;
- Cancelled: retain during the retention window;
- Done: retain during the configured source-retention window;
- deleted job: source becomes immediately eligible for cleanup.

Default source retention is 24 hours, using a dedicated configuration value rather than conflating it with Markdown-result retention.
This keeps failed/restarted jobs genuinely retryable while bounding storage.

On process startup, active jobs become Interrupted as today.
Cleanup must consider every still-referenced uploaded source, not only active jobs.
A referenced source inside its retention window cannot be removed.

Retry continues to reuse the same persisted job/source and increments attempt count.

For LIVE jobs, retained final audio also backs the client download endpoint during the retention window.
## Audio download and client save

Expose a guarded job-audio endpoint for LIVE-upload jobs while their retained source exists.

Requirements:
- job must reference an uploaded LIVE source;
- source must resolve inside the ScribeWatch upload storage root;
- file must exist;
- response has an attachment filename derived from the LIVE timestamp/session name.

After HTTP 202:
- Server-folder mode may clear browser recovery immediately because the durable requested copy already exists and the server source is retained.
- None mode may clear recovery after durable server acceptance because the server source is retained for retry.
- This-computer mode clears recovery only after the finalized audio has also been written/downloaded successfully to the chosen client destination.

If a client copy fails or permission is lost, the job remains accepted and the browser recovery remains available.
The user gets a retry-save action; transcription may proceed independently.
## STT timeout hardening

Provider `timeoutSeconds` must apply to actual transcription HTTP requests.
The simplest implementation is a total request timeout on the reqwest RequestBuilder.

A route-level `fallbackAfterSeconds` remains an independent, potentially shorter, fallback deadline.
Whichever deadline fires first ends that route attempt.

Expected semantics:
- provider request timeout -> failed attempt with a clear timeout error;
- route fallback timeout -> TimedOut outcome and continue to next route;
- cancellation -> cancelled job path;
- no route may hang indefinitely.

The currently configured OmniRoute Transcription timeout of 600 seconds therefore becomes effective rather than descriptive only.
## Durability and privacy details

Browser:
- checkpoint writes are serialized;
- manifest update happens only after chunk write succeeds;
- OPFS recovery is never silently discarded after an error;
- unsupported persistence APIs degrade with explicit warnings.

Server:
- uploaded files use private permissions where supported;
- persistent data lives under the existing local ScribeWatch data mount;
- SQLite remains local with WAL and synchronous=FULL;
- Markdown publication semantics remain unchanged;
- no new public listener or network port is introduced.

No attempt is made to encrypt browser OPFS independently of the browser profile or to introduce medical-record encryption in this feature.
That is a separate security/compliance project if required.

Server and browser cleanup must never touch user-selected permanent audio copies.
## Failure matrix

| Failure | Required result |
| --- | --- |
| Browser tab crash | Last durably checkpointed chunks remain in OPFS |
| Full reload | Session appears as recoverable/interrupted |
| In-app navigation | Recording continues |
| Browser hidden/minimized | Force requestData checkpoint; continue recording if platform permits |
| Microphone unplugged | Preserve chunks, mark interrupted, offer Continue |
| Permission revoked | Preserve chunks, mark interrupted |
| OPFS write fails | Stop safely, report blocking durability error, keep previously written chunks |
| Upload network failure | Keep OPFS session; offer Retry Upload |
| ScribeWatch crashes during upload | No accepted job; browser can resend |
| ScribeWatch crashes after HTTP 202 | Job/source survive; job becomes Interrupted and Retry works |
| STT fails | Server audio source remains retryable |
| Server-folder copy fails | Do not claim acceptance; retain server staging and browser recovery |
| Client-folder copy fails | Keep OPFS recovery and offer Retry Save |
| FFmpeg segment assembly fails | Keep staged segments within retention window |
| User deletes recovery | Explicit destructive confirmation |

## UI constraints

Keep the existing LIVE page visually simple.
The primary recording control remains a single Record/Stop control.
Recovery appears as a prominent card only when relevant.
Storage/persistence diagnostics are compact, not a separate settings system.

During an active session show:
- elapsed duration;
- durable-save indicator;
- locally protected bytes;
- persistence status;
- microphone state.

Do not expose internal chunk terminology in normal success-path UI.
## Testing strategy

Frontend unit tests use an injected storage adapter with an in-memory implementation rather than mocking browser internals ad hoc.

Required frontend tests:
1. chunk is durable before in-memory reference is released;
2. crash-shaped manifest is rediscovered as Interrupted;
3. Continue creates segment 2 under the same session;
4. interruption gap is retained and not converted into fake audio;
5. failed upload keeps recovery;
6. successful accepted upload clears recovery according to retention mode;
7. client-save failure keeps recovery;
8. recorder error and track-ended preserve session;
9. controller survives LiveRecordView destruction/navigation;
10. seek-to-last-30-seconds clamps correctly;
11. unsupported directory picker uses download fallback.

Required Rust/backend tests:
1. LIVE multipart streams multiple segments without loading them as one buffer;
2. invalid/non-audio segment is rejected;
3. segment order is manifest-controlled;
4. server-folder final M4A is published before job enqueue;
5. failed STT preserves uploaded source;
6. Interrupted job preserves source across AppState reload and cleanup;
7. Retry after restart can reuse source;
8. terminal source expires only after retention;
9. deleted job source becomes cleanup-eligible;
10. provider timeout is actually enforced;
11. route fallback can still beat provider timeout;
12. audio download rejects paths outside upload storage.
## Integration and smoke verification

Automated gates:
- `cargo test --locked --all-targets`;
- frontend `npm test`;
- frontend `npm run check`;
- frontend `npm run build`;
- Docker production build.

Manual/synthetic crash-path verification on Cloud9:
1. start a LIVE recording and confirm OPFS protected-byte count grows;
2. reload the page and confirm recovery appears;
3. seek near the end and verify playback;
4. Continue, record a second segment, then Stop;
5. finalize to a server folder and verify one playable M4A;
6. force an upload failure and verify local recovery remains;
7. force an STT failure and verify Retry source remains;
8. restart ScribeWatch after acceptance and verify Interrupted + Retry;
9. verify no recovery/audio is silently deleted;
10. verify normal short LIVE recording remains simple.

Production deployment must retain the immediately previous known-good Docker image as rollback.
Temporary build/test containers, target trees, node_modules, and stale staging created by tests are removed after verification.

## Code boundaries

Expected new focused modules:
- `frontend/src/lib/live-recovery.ts`: durable session model/storage adapter;
- `frontend/src/lib/live-recorder.ts`: long-lived recorder/controller;
- `src/live.rs`: server LIVE finalization helpers;
- `src/api/live.rs`: LIVE multipart API.

Expected surgical edits:
- `LiveRecordView.svelte`: UI only;
- `api.ts` and types: LIVE calls/types;
- `quick.rs` / `pipeline_quick.rs`: retained upload lifecycle;
- `api/jobs.rs`: audio download;
- `provider.rs`: effective provider timeout;
- config/domain/router/tests/docs as required.

No broad refactor outside these boundaries unless tests reveal a shared correctness bug.
## Acceptance criteria

The feature is complete only when all of the following are true:

- A multi-hour LIVE recording does not accumulate the full audio in JS heap memory.
- Reload/crash recovery demonstrably restores the already checkpointed audio.
- The user can listen/scrub the latest pre-crash audio before choosing Continue.
- Continue appends a new logical segment and preserves a visible interruption gap.
- A completed session becomes one playable final M4A.
- The user can choose None, Server folder, or This computer for final-audio retention.
- Failed client saving never destroys the recovery copy.
- Failed upload never destroys the recovery copy.
- Failed/interrupted STT never destroys the server source before retention expiry.
- Retry after ScribeWatch restart has a real source file.
- Provider timeout configuration is effective.
- Internal navigation does not stop an active LIVE capture.
- Existing Quick, Workflow, export, archive, and recursive-watch behavior remains green.
- Production health/API checks pass after deployment.
- Git working tree, GitHub main, and deployed source are synchronized.
- Cleanup leaves only the active image plus one immediate rollback image and no transient test/build artifacts.
