<p align="center">
  <img src="docs/logo.svg" width="128" alt="ScribeWatch logo">
</p>

<h1 align="center">ScribeWatch</h1>

<p align="center">
  <strong>Voice in. Markdown out. Automatically.</strong><br>
  Self-hosted voice-note transcription for Obsidian, Markdown folders and automated audio workflows.
</p>

<p align="center">
  <a href="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/GodsQuantum/ScribeWatch/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="MIT license" src="https://img.shields.io/badge/license-MIT-3dd7cf"></a>
  <img alt="Rust" src="https://img.shields.io/badge/backend-Rust-ef7d57">
  <img alt="SvelteKit" src="https://img.shields.io/badge/UI-SvelteKit-ff3e00">
  <img alt="Docker" src="https://img.shields.io/badge/self--hosted-Docker-2496ed">
  <img alt="Obsidian friendly" src="https://img.shields.io/badge/Obsidian-friendly-7c3aed">
</p>

<p align="center">🇫🇷 <a href="README.fr.md">README en français</a></p>

---

ScribeWatch turns recordings into **clean, titled Markdown notes** using your own OpenAI-compatible speech-to-text provider. Drop one voice memo into **Quick Transcribe**, or point a **Workflow** at a folder and let ScribeWatch handle new recordings automatically.

It is especially useful as a **voice notes → Obsidian** bridge, but nothing is Obsidian-specific: any Markdown folder works.

<p align="center">
  <img src="docs/screenshots/home-quick-transcribe.png" width="100%" alt="ScribeWatch dashboard and Quick Transcribe">
</p>

## ✨ Why ScribeWatch?

- **Voice notes become useful files** — readable `.md`, not another pile of forgotten recordings.
- **Quick Transcribe** — drag in one audio file and get Markdown back immediately.
- **Watch folders** — automatically process new recordings from a server, NAS or mounted sync folder.
- **Obsidian-friendly by default** — clean filenames, H1 titles, YAML properties and tags.
- **Bring your own transcription engine** — works with OpenAI-compatible STT endpoints such as Speaches/Whisper services.
- **No LLM required** — titles and Markdown formatting are deterministic; the transcript stays authoritative.
- **Safe publication** — Markdown is written before workflow audio is archived, and existing notes are never silently overwritten.
- **Small self-hosted stack** — Rust/Axum backend, SvelteKit UI, SQLite state, one container.

## 🎙️ Two ways to use it

| | **Quick Transcribe** | **Watch folders** |
|---|---|---|
| Best for | One voice memo right now | Recurring / automatic capture |
| Input | Browser upload or server file | Watched server/NAS folder |
| Output | Server folder or your computer | Markdown folder |
| Original audio | Never moved | Archived only after Markdown succeeds |
| Typical use | “I just recorded an idea” | Phone/sync folder → Obsidian automatically |

<p align="center">
  <img src="docs/screenshots/quick-transcribe.png" width="49%" alt="ScribeWatch Quick Transcribe">
  <img src="docs/screenshots/workflow-folders.png" width="49%" alt="ScribeWatch workflow folders">
</p>

## 🚀 Quick start

Requirements: Docker Engine + Compose and an OpenAI-compatible transcription endpoint.

```bash
git clone https://github.com/GodsQuantum/ScribeWatch.git
cd ScribeWatch
cp .env.example .env
cp compose.example.yaml compose.yaml
mkdir -p config data watch notes archive

docker compose up --build -d
```

Open **http://127.0.0.1:3000**, then:

1. Add your transcription endpoint under **Providers**.
2. Drop a recording into **Quick Transcribe** — or create a Workflow.
3. For automation, choose **Watch folder → Markdown folder → Audio archive**.
4. Point the Markdown folder at your Obsidian vault, synced notes folder, NAS, or any other Markdown destination.

> By default ScribeWatch binds locally. If you expose it on a trusted LAN/VPN, change `SCRIBEWATCH_BIND_HOST` and use your normal firewall or authenticated reverse proxy policy.

## 🧠 Built for Obsidian — not locked to it

A transcript can become a note like this:

```markdown
---
title: "Remember to book the train tomorrow"
created: 2026-09-16T08:42:00Z
tags:
  - voice-note
  - transcription
  - scribewatch
source: "Recording 42.m4a"
language: "en"
---

# Remember to book the train tomorrow

Remember to book the train tomorrow...
```

ScribeWatch derives the title from the transcript, keeps Unicode-readable filenames, deduplicates tags and handles filename collisions as `Title (2).md`, `Title (3).md`, etc.

<p align="center">
  <img src="docs/screenshots/mobile-quick-transcribe.png" width="390" alt="ScribeWatch Quick Transcribe on mobile">
</p>

## 🔄 Workflow model

```text
Voice recorder / sync / NAS
            │
            ▼
       Watch folder
            │
       transcription
            │
            ▼
      Markdown folder ─────► Obsidian / notes / knowledge base
            │
     publish succeeds
            │
            ▼
       Audio archive
```

Workflow audio is **never archived before Markdown publication succeeds**. Native filesystem events are backed by periodic reconciliation, which also helps with network-mounted folders where events may be missed.

## 🔒 Safety by design

- Provider API keys stay server-side and are never returned to the browser.
- Server browsing is restricted to explicit allowed roots and rejects symlink escapes.
- Browser uploads are bounded while streaming and cleaned after terminal job states.
- Quick Transcribe never moves or deletes the original source file.
- Existing Markdown is never silently overwritten.
- Interrupted work is recorded explicitly after restart instead of pretending it completed.
- The example container runs non-root with a read-only root filesystem, dropped capabilities and `no-new-privileges`.

See [`SECURITY.md`](.github/SECURITY.md).

## ⚙️ Configuration

| Variable | Default | Purpose |
|---|---|---|
| `SCRIBEWATCH_HOST` | `0.0.0.0` | HTTP bind address inside the container |
| `SCRIBEWATCH_PORT` | `3000` | HTTP port |
| `SCRIBEWATCH_CONFIG_DIR` | `/config` | SQLite/config directory |
| `SCRIBEWATCH_DATA_DIR` | `/data` | Quick upload/result staging |
| `SCRIBEWATCH_ALLOWED_ROOTS` | required | Colon-separated server roots ScribeWatch may access |
| `SCRIBEWATCH_SCAN_SECONDS` | `5` | Watch-folder reconciliation cadence |
| `SCRIBEWATCH_FILE_STABILITY_MS` | `2000` | Stable-size/mtime window before ingest |
| `SCRIBEWATCH_MAX_TRANSCRIPTION_JOBS` | `2` | Concurrent transcription jobs |
| `SCRIBEWATCH_MAX_UPLOAD_BYTES` | `2147483648` | Maximum streamed browser upload |
| `SCRIBEWATCH_QUICK_RESULT_RETENTION_HOURS` | `24` | Client-result retention window |

Supported audio candidates: `mp3`, `wav`, `m4a`, `flac`, `ogg`, `opus`, `aac`, `wma`, `aiff`, `aif`, `caf`, `webm`.

## 🧩 Client-computer folders

Browsers cannot continuously watch arbitrary folders on another computer. ScribeWatch keeps that boundary explicit:

- local audio enters through drag-and-drop or the browser file picker;
- completed Markdown can be written directly to a local folder when the browser supports the File System Access API in a secure context;
- otherwise ScribeWatch downloads the `.md` normally.

No browser directory handle is sent to or stored by the ScribeWatch server.

## 🛠️ Stack & development

**Backend:** Rust 1.98 · Axum · Tokio · SQLite · notify
**Frontend:** Svelte 5 · SvelteKit 2 · TypeScript

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cd frontend && npm ci && npm test && npm run check && npm run build
```

Deterministic end-to-end check:

```bash
bash scripts/e2e-v02.sh
```

## 📡 API highlights

- `POST /api/v1/quick/upload` — streamed browser upload
- `POST /api/v1/quick/server` — allowed server-side audio file
- `GET /api/v1/jobs/{id}/markdown` — completed client-output Markdown
- `GET|POST /api/v1/workflows` — watch-folder automation
- `GET|POST /api/v1/providers` — OpenAI-compatible transcription providers
- `GET /api/v1/jobs`, `GET /api/v1/events` — history and live state
- `GET /api/v1/health`, `GET /api/v1/ready` — health/readiness

## 🤝 Contributing

Issues and pull requests are welcome. See [`CONTRIBUTING.md`](.github/CONTRIBUTING.md) and [`CODE_OF_CONDUCT.md`](.github/CODE_OF_CONDUCT.md).

If ScribeWatch earns a place in your stack, a ⭐ helps other voice-note and Obsidian users find it.

## 📄 License

[MIT](LICENSE)
