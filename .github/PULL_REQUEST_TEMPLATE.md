## What changed

Describe the user-visible or technical change.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-targets`
- [ ] `cd frontend && npm ci && npm test && npm run check && npm run build`
- [ ] Container build / relevant watch→Markdown→archive smoke test

## Pipeline behavior

If this changes watching, stability detection, transcription, Markdown publication, archival, retries, or path validation, describe the regression case and expected failure/success semantics.
