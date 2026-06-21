# Final Rereview 2 Fix Report

## 2026-06-21 20:15:29 CEST - Clear Displayed Fallback Selection

Changed behavior:
- `DeviceSelection` now exposes `displayed_selection_is_fallback()` so callers can tell when the displayed row is only a fallback for a remembered missing device.
- The desktop MIDI input combo box now commits an explicitly clicked displayed fallback row, including `<none selected>`, even when its index did not change.
- Regression coverage now selects device `a`, refreshes while `a` is absent, explicitly selects index `0` / `MidiInputPort::None`, refreshes with `a` present again, and verifies the remembered and displayed selection remain `None`.

Verification:
- `cargo test -p desktop device_selection`: passed, 4 tests.
- `cargo test -p desktop`: passed, 11 tests plus desktop bin and doctests.
- `cargo check -p desktop`: passed.
- `./scripts/dev.sh clippy-all`: passed.
- `cargo +nightly fmt --all -- --check`: passed.
- `git diff --check`: passed.
