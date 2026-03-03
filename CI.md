# Running CI Locally

This project uses a combination of a local xtask and GitHub Actions for CI.

## Local CI Commands

### Quick Check (Recommended)

Runs native and WASM clippy with the most common feature combinations:

```bash
cargo run -p ci -- clippy-all
```

This checks:
- Native (all features)
- Native (`on_off_widgets` feature for gui crate)
- WASM32 (all features)

### Exhaustive Feature Testing

Requires `cargo-hack` (install once with `cargo install cargo-hack`):

```bash
cargo run -p ci -- clippy-hack
```

This runs clippy for every feature combination on both native and WASM targets.

## GitHub Actions

A CI workflow is also configured at [`.github/workflows/clippy.yml`](.github/workflows/clippy.yml) that runs on every push and pull request. It tests:

| OS | Target | Features |
|----|--------|----------|
| macOS | x86_64-apple-darwin | all |
| macOS | aarch64-apple-darwin | all |
| Linux | x86_64-unknown-linux-gnu | all |
| Linux | x86_64-unknown-linux-gnu | on_off_widgets |
| Linux | wasm32-unknown-unknown | all |

## Requirements

- Rust stable toolchain
- For WASM testing: `rustup target add wasm32-unknown-unknown` (automatic if missing)
- For exhaustive testing: `cargo install cargo-hack`
