# ScribeWatch desktop installer candidates

Sources: native Rust 1.98 + existing SvelteKit. This packaging wraps the real server,
without Docker, Python or a separate browser extension. The launcher opens the app
in the system's default browser and runs a loopback-only service.

- Windows 11 AMD64: NSIS per-user setup, includes backend, frontend,
  FFmpeg/FFprobe 9.0.2, license, shortcut, clean uninstaller. Keeps user data.
- macOS 15 Apple Silicon and Intel: .app in a mounted .dmg with all resources.
  The candidate uses **ad-hoc** signing only. Production Apple distribution
  requires a Developer ID certificate, hardened signing and notarization.
- Persistent data: Windows LOCALAPPDATA/ScribeWatch; macOS
  ~/Library/Application Support/ScribeWatch. SQLite remains on local storage.
- The PATH contains only the bundled FFmpeg first, no system PATH mutation.
- Installer checks enforce FFmpeg codecs; AutoSubs additionally requires libass.
- Tests build/assemble the installer on the actual target OS, unpack/install,
  verify launcher's --check-install, launch a real Rust backend with --smoke,
  and exercise clean uninstallation on Windows.
- FFmpeg is fetched from pinned upstream v9.0.2 assets by SHA256, never latest;
  bundling its original -L license terms and source URL.
- Unsigned preview artifacts are **NOT** certified as signed/notarized public
  binaries and should not be promoted into a stable release without signing
  credentials and Gatekeeper/SmartScreen testing.
- GPU end-to-end, real audio codec/transcript and complete UX/system tray
  require separate integration tests. CI runners may lack compatible GPUs.
- Cloud9-Speaches pairing is available in the existing app UI; it does
  not imply that Cloud9-Speaches has itself been packaged on Windows/macOS.

## ScribeWatch standalone with optional native AI engines (candidate)

The Windows x64 full standalone installer contains ScribeWatch, the verified
Cloud9-Speaches native transcribe.cpp/FastAPI executable, and upstream llama.cpp
native CPU/Vulkan/NVIDIA CUDA variants. Model weights are not included.
The previous slim ScribeWatch installer remains supported and unchanged.

**Fresh Windows full native installation:** first launch provisions local
Parakeet Ultra and Qwen3.5-4B Q4_K_M providers, connects all five built-in
report profiles and starts the existing embedded engines. The native model
APIs automatically request both model downloads into the user-owned cache.
Internet is required only for the initial model acquisition (several GB);
subsequent transcription, document editing and PDF export run locally.
REC reports model readiness and disables recording until both are available.
If interrupted, downloads can be resumed on the next launch or through
Advanced Settings. No extra service or periodic downloader is created.
Smoke CI, Docker, network-only deployments and existing configured profiles
are excluded to preserve user settings and external engines.

Advanced Settings still permit Internal, External and Disabled separately.
Native STT binds only to 127.0.0.1:38005 and LLM to 127.0.0.1:38080.
Switching to External/Disabled stops the owned child without deleting files.

The STT model UI reuses Cloud9-Speaches native model APIs and single HF cache:
Whisper, Parakeet Ultra, Qwen3-ASR, Nemotron and custom GGUF quantizations.
The optional pinned ScribeWatch acoustic pack contains the Whisper Turbo
model and French aligner with SHA checks. The LLM engine reuses upstream
llama.cpp router mode with --models-max 1, --models-memory-margin 512,
--sleep-idle-seconds 45 and an automatic hardware-aware context budget (24,576 tokens
with q4_0 KV cache on the validated 4 GiB PC). Windows attempts CUDA if the NVIDIA
driver is detected, then Vulkan, then CPU. Physical GPU support still
requires on-device verification. The load-time memory margin does not
measure unrelated external VRAM pressure; idle unloading happens natively.
Neither engine unloads the model in the middle of an active request.

The Windows full installer uses a compressed ZIP payload **inside the same
single setup.exe**, extracted by built-in Windows 11 tar.exe at installation.
This avoids upstream NSIS's fixed ~2 GiB mmap limit without using a separate
installer/data file or shipping another archive extractor. The slim installer
continues using the original direct NSIS file packaging. Both modes have
real native install/smoke/uninstall CI gates.

The native input assembler standalone_bundle.py accepts pre-downloaded,
pinned and SHA256-checked releases, rejects ZIP path traversal/symlinks,
and produces BUNDLED-RUNTIMES-SHA256.json. Setting SCRIBEWATCH_EMBEDDED_DIR
in the existing native build pipeline adds the exact verified binaries to
a special standalone installer; without it, the slim installer is unchanged.

Certification gates:
1. Rust, frontend, native manager, and secure archive assembler unit tests.
2. Actual ScribeWatch backend + fake engines on localhost; mode transitions,
   model discovery and preservation of external providers.
3. Actual Windows NSIS install, launcher smoke, start real packaged STT and
   LLM servers, verify endpoints, stop both and uninstall.
4. Physical microphone and GPU quality/VRAM/latency testing; code signing.

Do not claim the full installer ready until gate 3 is verified on an
exact pinned release source. Full packaging initially targets Windows x64.
Mac/Linux retain their existing slim builds until complete native bundles
pass their own independent CI. No patient materials are used in tests.

## Reproducible clinical engine contract

llama-runtime-lock.json is the reviewed source of truth for the exact
upstream engine commit, archive SHA-256s and the 4B clinical pilot. The
Windows standalone CI verifies all three actual engine versions before
packaging and tests the prompt hash, context policy, deterministic sampler,
source-partitioning and local source-grounded recall.

The successful private reference was Qwen3.5-4B Q4_K_M and llama.cpp v0.9.0
(cd959bae9), 24,576-token context, q4_0 KV, non-thinking,
temperature 0.1, top_p 0.9, output ceiling 5,000 tokens, stable input seed.
The original patient data/reference PDF are never included in Git or CI.

A passing installer test is NOT an E2E medical quality test: GPU kernels,
different backend versions, model identifiers and OS can change results.
A new machine must run a real private consultation regression and receive
clinical review before quality is certified. Other OS builds that do not
embed llama.cpp must not claim to use this pinned runtime.
