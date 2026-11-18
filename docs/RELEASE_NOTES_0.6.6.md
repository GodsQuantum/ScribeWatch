# ScribeWatch 0.6.6 — 18 November 2026

## Dictation and resilience

- Native global Rust microphone dictation on Windows, macOS and Linux, using the MIT-licensed Handy recorder core. Audio streams to a temporary WAV file, avoiding a fixed duration limit or retaining the entire recording in RAM.
- Select local or remote ASR provider, model and fallback chain. Configure optional AI structuring with local or remote LLM and document profiles; the raw recognized transcript is preserved if the optional structurer fails. Original browser REC live session, automatic recording recovery, and quick audio-to-text remain available.
- If the HTTP port preferred by the installed desktop app (default 3052, or SCRIBEWATCH_PORT) is occupied, the application automatically selects a free loopback-only port. The bundled STT (38005) and LLM (38080) follow the same policy. Only healthy ScribeWatch sessions are reused; other applications and custom external providers are never stopped or overwritten.
- The app stores the actual healthy main port for later launches and dynamically updates ScribeWatch-owned native provider URLs if those ports change.
- Compatible Windows x64, macOS ARM64/Intel, Linux x86-64/ARM64 and Docker/server binaries are built from a single tagged source tree.

## Verification and boundaries

- Branch PR #52: Linux native Rust unit suite (194 passed, one ignored), first-boot integration (3), real launcher/engine port-collision integration (1), launcher unit (4), and frontend tests (120) passed locally, with strict Clippy and zero Svelte diagnostics.
- Release GitHub Actions must pass all platform and installer jobs, verify final checksums and only then publish the version as non-draft. Hardware microphone/keyboard, individual GPU models, Apple notarization and Windows Authenticode signing are not universally certified.
- The full local edition obtains ASR and LLM model weights on first launch; no hidden cloud inference is enabled. Lightweight builds may require external providers. Existing configurations are retained.

See README.md, README.fr.md and README.zh-CN.md for downloads and limitations.
