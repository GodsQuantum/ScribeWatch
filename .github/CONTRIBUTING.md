# Contributing

ScribeWatch keeps business rules in Rust and the SvelteKit UI as a thin API client. The core invariant is immutable: audio can be archived only after its Markdown note is safely published.

Before submitting a change:
```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cd frontend && npm ci && npm test && npm run check && npm run build
```

For filesystem/provider changes, add regression tests for failure semantics and no-overwrite behavior. Keep secrets out of fixtures, logs and screenshots.
