# ScribeWatch Linux AppImage (native-server preview)

This is the actual ScribeWatch Rust server and Svelte UI in a Linux AppImage,
not the separate localhost microphone relay and not a Docker launcher.

- Linux x86_64 and aarch64; native CPU executable, bundled FFmpeg/FFprobe,
  curl, fontconfig tools and pinned CA certificates.
- Persistent state under XDG_CONFIG_HOME/scribewatch, XDG_DATA_HOME/scribewatch
  and XDG_STATE_HOME/scribewatch; server bound to 127.0.0.1:3052 by default.
- Use `--no-browser` for a foreground server without opening the default browser.
  `--version` and `--help` are forwarded to the bundled Rust binary.
- SCRIBEWATCH_ALLOWED_ROOTS defaults to the user's home and data folders plus
  mounted media roots present on the machine. No access to remote user data by default.
- Transcription can use Cloud9-Speaches over the existing ScribeWatch provider API.
- PDF and other advanced export formats that require Pandoc/WeasyPrint may need
  those tools installed on the host. Do not claim full offline export parity or
  a production-ready universal installer until each feature is certified.
- The AppImage does not embed GPU ASR weights or Cloud9-Speaches itself.

Build: first `npm ci && npm run build` inside frontend; then
`cargo build --release --locked --target <arch>-unknown-linux-musl`, and
`SCRIBEWATCH_BINARY=target/<arch>-unknown-linux-musl/release/scribewatch
packaging/appimage/build.sh <arch>`. Pinned download hashes are defined in
`fetch-tools.sh`.
