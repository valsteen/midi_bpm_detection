# Final Review Fix Report

## 2026-06-21 Native Desktop Final Review Fix

### Changes

- Added a desktop controller command scheduler that runs MIDI service commands on short-lived background threads and logs command/spawn failures.
- Changed desktop GUI MIDI controls to use `try_lock`, snapshot device selection state, and schedule refresh/select commands outside egui rendering.
- Changed static and dynamic config callbacks to schedule controller updates instead of running blocking MIDI service work inline.
- Refreshed MIDI input devices once during startup before storing the controller for the first GUI frame.
- Changed the macOS device-change callback to schedule a background device refresh and request repaint after the refresh attempt.
- Removed the unused `tokio` dependency from `crates/desktop/Cargo.toml`.
- Updated the `gui::start_gui` shutdown comment for direct desktop usage.
- Added focused non-visual tests for the background slot command scheduler.

### Verification

- `cargo check -p desktop`: passed.
- `cargo test -p desktop`: passed, 10 tests.
- `./scripts/dev.sh clippy-all`: passed.
- `scripts/dev.sh check-plugin`: passed.
- `scripts/dev.sh check-wasm`: passed.
- `scripts/dev.sh clippy-wasm`: passed.
- `cargo +nightly fmt --all -- --check`: passed.
- `git diff --check`: passed.
