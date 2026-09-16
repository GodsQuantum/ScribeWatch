<p align="center">
  <img src="docs/logo.svg" width="128" alt="ScribeWatch logo">
</p>
<h1 align="center">ScribeWatch</h1>
<p align="center"><strong>Turn voice notes into Markdown before you forget them.</strong></p>
<p align="center">
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-5ee0bb"></a>
  <img alt="Rust" src="https://img.shields.io/badge/backend-Rust-5ee0bb">
  <img alt="SvelteKit" src="https://img.shields.io/badge/frontend-SvelteKit-5ee0bb">
  <img alt="self-hosted" src="https://img.shields.io/badge/self--hosted-yes-5ee0bb">
</p>
<p align="center">🇫🇷 <a href="README.fr.md">README en français</a></p>

ScribeWatch is a small self-hosted app for turning recordings into clean, titled Markdown notes with your own OpenAI-compatible speech-to-text provider. It works especially well as a fast **voice note → Obsidian vault** bridge, while remaining useful with any Markdown folder.

## Two ways to capture ideas

**Quick Transcribe** handles one file right now: drag audio from your computer or pick an allowed file already on the server, transcribe it, then save the resulting `.md` to a server folder or back to your computer.

**Watch folders** automate the routine: ScribeWatch waits for stable audio, transcribes it, publishes Markdown to the folder you choose, and archives the original audio only after publication succeeds.
## What it looks like

![ScribeWatch home dashboard](docs/screenshots/home-quick-transcribe.png)

<details>
<summary>Quick Transcribe and workflow editor</summary>

![Quick Transcribe desktop](docs/screenshots/quick-transcribe.png)

![Workflow editor with Watch, Markdown and Archive folders](docs/screenshots/workflow-folders.png)

<img src="docs/screenshots/mobile-quick-transcribe.png" width="390" alt="Quick Transcribe on mobile">
</details>

## Why it works well with Obsidian

Generated notes use the first useful words of the transcript as both the H1 and filename. YAML properties can include `title`, `created`, `tags`, source audio, workflow, provider, model and language. Default tags are `voice-note`, `transcription` and `scribewatch`, with optional workflow tags added on top.

No LLM is required for titling or formatting. Your transcript stays authoritative.
## Quick start with Docker Compose

```bash
cp .env.example .env
mkdir -p config data watch notes archive
cp compose.example.yaml compose.yaml
docker compose up --build -d
```

Open `http://127.0.0.1:3000` with the defaults. To expose it on a trusted LAN/VPN, change `SCRIBEWATCH_BIND_HOST` and apply your normal firewall or authenticated reverse-proxy policy.

Then:

1. Add an OpenAI-compatible transcription endpoint under **Providers**.
2. Use **Quick Transcribe** immediately, or create a workflow.
3. For a workflow, choose **Watch folder**, **Markdown folder** and **Audio archive** independently.
4. Leave Markdown folder empty if you want legacy behavior: notes are written beside the source audio.

The archive must remain outside the watched tree. `/config` should use local storage because it contains SQLite; `/data` stores bounded Quick Transcribe staging/results.
## Core guarantees

- Workflow audio is archived only after its Markdown note is durably published.
- Existing Markdown is never silently overwritten; title collisions become `Title (2).md`, `Title (3).md`, and so on.
- Quick Transcribe never moves or deletes the original source file.
- Browser uploads are staged under `/data`, bounded while streaming, and cleaned after terminal job states.
- Provider API keys stay server-side and are never returned to the browser.
- Server browsing is constrained to explicit allowed roots and rejects symlink escapes.
- Active jobs survive as explicit `interrupted` history after restart instead of pretending completion.

## Client-computer folders

A remote web server cannot continuously watch arbitrary folders on your laptop or desktop. ScribeWatch keeps that distinction explicit:

- local audio enters through drag-and-drop or the file picker;
- completed Markdown can be saved directly to a local folder when the browser supports the File System Access API in a secure context;
- otherwise ScribeWatch downloads the `.md` normally.

No browser directory handle is sent to or stored by the ScribeWatch server.
## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `SCRIBEWATCH_HOST` | `0.0.0.0` | HTTP bind address inside the container |
| `SCRIBEWATCH_PORT` | `3000` | HTTP port |
| `SCRIBEWATCH_CONFIG_DIR` | `/config` | Local SQLite/config directory |
| `SCRIBEWATCH_DATA_DIR` | `/data` | Quick upload/result staging |
| `SCRIBEWATCH_ALLOWED_ROOTS` | required | Colon-separated canonical server roots |
| `SCRIBEWATCH_SCAN_SECONDS` | `5` | Watch-folder reconciliation cadence |
| `SCRIBEWATCH_FILE_STABILITY_MS` | `2000` | Stability observation window |
| `SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS` | `2` | Maximum concurrent transcription jobs |
| `SCRIBEWATCH_MAX_UPLOAD_BYTES` | `2147483648` | Maximum streamed browser upload size |
| `SCRIBEWATCH_QUICK_RESULT_RETENTION_HOURS` | `24` | Client-result retention window |

Supported audio candidates: `mp3`, `wav`, `m4a`, `flac`, `ogg`, `opus`, `aac`, `wma`, `aiff`, `aif`, `caf`, `webm`.

## Stack

Rust 1.98 + Axum/Tokio own filesystem security, durable job state, transcription, publication and archival. SQLite stores local state and deduplication fingerprints. Svelte 5 + SvelteKit 2 provide the static responsive UI. `notify` events are backed by periodic reconciliation for network-mounted watch folders.
## API highlights

- `POST /api/v1/quick/upload` — one streamed browser upload
- `POST /api/v1/quick/server` — one allowed server-side audio file
- `GET /api/v1/jobs/{id}/markdown` — completed client-output Markdown
- `GET|POST /api/v1/workflows` — watch-folder automation
- `GET|POST /api/v1/providers` — OpenAI-compatible transcription providers
- `GET /api/v1/jobs`, `GET /api/v1/events` — history and live state
- `GET /api/v1/health`, `GET /api/v1/ready` — health/readiness

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cd frontend && npm ci && npm test && npm run check && npm run build
```

For a deterministic local end-to-end check, run `bash scripts/e2e-v02.sh`.

ScribeWatch was derived from AutoSubs' watcher/job foundations; the history is intentionally preserved so that lineage remains auditable. Licensed under MIT.