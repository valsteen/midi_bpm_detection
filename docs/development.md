# Development Commands

This project has three main build modes:

- `desktop`: the native TUI/GUI application in `crates/tui`, sharing the `gui` crate.
- `plugin`: the CLAP/VST3 plugin in `crates/midi-bpm-detector-plugin`.
- `wasm`: the browser demo in `crates/wasm`.

The root workspace default members are `tui` and `midi-reset`. The plugin and wasm crates should be checked explicitly.

## One Command Surface

Use the helper script when you do not remember the exact Cargo invocation:

```shell
scripts/dev.sh help
```

The script does not hide the underlying commands. It exists to keep the command list in one repo-owned place instead of
scattering it across IDE run configurations.

## Local Setup

The project uses the stable Rust toolchain for builds and checks, plus nightly rustfmt for formatting. Install the Rust
components from `rust-toolchain.toml` with:

```shell
rustup component add clippy rustfmt rust-src
rustup toolchain install nightly --component rustfmt
```

Check the local setup with:

```shell
scripts/dev.sh doctor
```

For WASM work, install:

```shell
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install -f wasm-bindgen-cli --version 0.2.125
```

`wasm-bindgen-cli` must match the `wasm-bindgen` version resolved by Cargo. A mismatch shows up as a bindgen schema
error when running `cargo test -p wasm --target wasm32-unknown-unknown`.

Check only the WASM setup with:

```shell
scripts/dev.sh doctor-wasm
```

## Formatting

Formatting intentionally uses nightly rustfmt:

```shell
scripts/dev.sh fmt
scripts/dev.sh fmt-check
```

Equivalent commands:

```shell
cargo +nightly fmt --all
cargo +nightly fmt --all -- --check
```

The project toolchain in `rust-toolchain.toml` is stable, but `rustfmt.toml` uses nightly-only rustfmt options such as
`format_strings`, grouped imports, and import granularity. Stable rustfmt will warn about those options and ignore them.

## Native Desktop/TUI

Check:

```shell
scripts/dev.sh check-desktop
```

Run tests:

```shell
scripts/dev.sh test-desktop
```

Run with local development config/data directories:

```shell
scripts/dev.sh run-desktop
```

Equivalent commands:

```shell
cargo check -p tui
cargo test -p tui
BPM_DETECTION_CONFIG=.data BPM_DETECTION_DATA=.data MIDI_TUI_CONFIG=.config MIDI_TUI_DATA=.data MIDI_TUI_LOG_LEVEL=info cargo run -p tui --bin bpm_detector_tui
```

## Plugin

Check the plugin crate:

```shell
scripts/dev.sh check-plugin
```

Bundle CLAP/VST3 artifacts:

```shell
scripts/dev.sh bundle-plugin
```

Equivalent commands:

```shell
cargo check -p midi-bpm-detector-plugin
cargo xtask bundle midi-bpm-detector-plugin --release
```

Bundled plugin artifacts are written under `target/bundled`.

## MIDI Reset Utility

Check the macOS-only MIDI reset command:

```shell
scripts/dev.sh check-reset
```

Equivalent command:

```shell
cargo check -p midi-reset
```

Running `cargo run -p midi-reset` restarts CoreMIDI on macOS.

## Clippy

Native checks:

```shell
scripts/dev.sh clippy-desktop
scripts/dev.sh clippy-plugin
scripts/dev.sh clippy-reset
scripts/dev.sh clippy-native
scripts/dev.sh clippy-all
```

Equivalent combined command:

```shell
cargo clippy -p tui -p midi-bpm-detector-plugin -p midi-reset --all-targets
cargo clippy -p wasm --target wasm32-unknown-unknown
```

`clippy-all` runs both `clippy-native` and `clippy-wasm`. Use it when you want to lint every supported build mode from
one command.

The workspace enables `clippy::pedantic` as warnings. Treat Clippy warnings as issues to fix by default. If a lint pushes
the code toward an unnatural shape or is wrong for the local context, prefer a narrow `#[allow(...)]` with a short reason
near the affected code instead of disabling the lint broadly.

Add `-- -D warnings` manually when you want CI-style strictness.

## Native Verification

For the usual native pre-commit pass:

```shell
scripts/dev.sh verify-native
```

This runs:

```shell
scripts/dev.sh fmt-check
scripts/dev.sh test-native
scripts/dev.sh check-native
scripts/dev.sh clippy-native
```

## WASM

Install the target and local tools once:

```shell
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install -f wasm-bindgen-cli --version 0.2.125
```

Intended commands:

```shell
scripts/dev.sh check-wasm
scripts/dev.sh test-wasm
scripts/dev.sh clippy-wasm
scripts/dev.sh build-wasm
scripts/dev.sh serve-wasm
scripts/dev.sh verify-wasm
```

Equivalent commands:

```shell
cargo check -p wasm --target wasm32-unknown-unknown
cargo test -p wasm --target wasm32-unknown-unknown
cargo clippy -p wasm --target wasm32-unknown-unknown
cd crates/wasm && NO_COLOR=false trunk build
cd crates/wasm && NO_COLOR=false trunk serve --port 8080 --open false
```

Trunk needs `NO_COLOR=false` in shells that export `NO_COLOR=1`, because Trunk `0.21.14` expects a boolean value for its
`--no-color` option. The helper script sets this for the Trunk commands.

`verify-wasm` runs `doctor-wasm`, `fmt-check`, `check-wasm`, `test-wasm`, `clippy-wasm`, and `build-wasm`.

### Browser Check

For the local browser loop:

```shell
scripts/dev.sh serve-wasm
```

Then open:

```text
http://127.0.0.1:8080/midi_bpm_detection/#dev
```

The path comes from `crates/wasm/Trunk.toml`, which sets `public_url = "/midi_bpm_detection/"`. The `#dev` suffix matters
during local development because `crates/wasm/index.html` skips service-worker registration when the hash is `#dev`.
Without it, a previous service worker can keep serving cached WASM/JS assets.

Expected smoke check:

- The page title is `Midi beat detector`.
- The top text says to tap computer keyboard or MIDI device.
- The egui canvas fills the browser window.
- Tapping keyboard keys should not produce console errors.
- If the browser does not grant Web MIDI permission, `Access to MIDI devices not granted.` is expected in the console.

To use another port:

```shell
WASM_PORT=8081 scripts/dev.sh serve-wasm
```

## Useful Groups

Before committing native-only changes:

```shell
scripts/dev.sh verify-native
```

Before touching the web demo:

```shell
scripts/dev.sh verify-wasm
```

Before checking all lint targets:

```shell
scripts/dev.sh clippy-all
```

Before releasing/testing the plugin in a DAW:

```shell
scripts/dev.sh verify-plugin
```
