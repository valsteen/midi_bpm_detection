## Final Re-review Fix - 2026-06-21

### Changes
- Preserved the remembered MIDI input selection when a selected device temporarily disappears.
- Added `DeviceSelection::displayed_selection()` so the desktop UI can show the visible fallback separately from the remembered wanted device.
- Updated the device selection regression test to cover selected device disappears, visible selection falls back to `<none selected>` / index 0, and the same device is restored when it reappears.
- Updated manual MIDI refresh in desktop controls to request an egui repaint after the background refresh completes, including error paths logged by `spawn_controller_command`.

### Verification
- `cargo test -p desktop device_selection` - passed, 3 tests.
- `cargo test -p desktop` - passed, 10 unit tests and doc-tests.
- `cargo check -p desktop` - passed.
- `./scripts/dev.sh clippy-all` - passed.
- `cargo +nightly fmt --all -- --check` - passed.
- `git diff --check` - passed.
