# Desktop portability and release roadmap

Last reviewed against **ScribeWatch v0.6.2**, released November 12, 2026.

## Available now

- One shared Rust/SvelteKit UI with Home, REC, Documents and Settings, plus Simple/Advanced modes, five themes and English/French/Simplified Chinese localization.
- Windows x64 **full standalone** installer contains the native Parakeet Ultra and llama.cpp/Qwen3.5-4B runtime interfaces. On first launch it downloads model weights and provisions the five built-in report profiles. First download requires an internet connection and significant disk capacity.
- Windows x64 lightweight desktop installer, macOS Apple Silicon/Intel lightweight DMGs, Linux x86-64/ARM64 AppImages, and Docker container. These do not currently ship the same ready-to-use acoustic + LLM model bundle.
- Original transcript preservation, resilient browser recording, structured notes, PDF and other document exports, configurable local/external provider chains.
- GitHub Actions have exercised packaging/install/smoke for the full Windows installer, and built the lightweight platforms.

## Work in progress in main (not yet a release)

The next tagged release gates Docker `latest` and native installer publication on all four full-engine builds (macOS ARM/Intel and Linux x86-64/ARM64). **The release contract has not yet been certified by a tagged build.**

The new pinned `native-full-portable.yml` build pipeline prepares full local-AI
macOS DMGs and Linux AppImages/deb packages using Speaches Reloaded
and llama.cpp. It must pass host-native installation and engine tests
before the edition is promoted or the website downloads are changed.

The next desktop packaging pass is also adding complete native
Pandoc/Typst document exporters and Noto Sans CJK fonts with SHA-256
source verification. Every format must pass an actual installer smoke
test without system-wide dependencies; this is not retroactive to the
v0.6.2 downloadable installers.

## Not yet certified

- A fresh physical end-to-end microphone/GPU/large-model/PDF test on each supported hardware variant; production clinical quality must be independently validated and clinician-reviewed.
- Fully bundled zero-configuration native Parakeet + Qwen model engines on macOS, Linux and Docker.
- Official commercial code signatures/notarization and exhaustive SmartScreen/Gatekeeper testing.
- Finished cross-platform system tray, automatic driver fallback certification and hardware-specific performance tuning on all devices.

See packaging/desktop/README.md for engineering details and the latest GitHub release for verified binaries. This document does not promise future dates, and the private deployment handoff is not part of this public repository.
