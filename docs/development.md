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

Run with local development config/data directories:

```shell
scripts/dev.sh run-desktop
```

Equivalent commands:

```shell
cargo check -p tui
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
```

Equivalent combined command:

```shell
cargo clippy -p tui -p midi-bpm-detector-plugin -p midi-reset --all-targets
```

The workspace enables `clippy::pedantic` as warnings. Add `-- -D warnings` manually when you want CI-style strictness.

## WASM

Install the target once:

```shell
rustup target add wasm32-unknown-unknown
```

Intended commands:

```shell
scripts/dev.sh check-wasm
scripts/dev.sh clippy-wasm
scripts/dev.sh build-wasm
```

Equivalent commands:

```shell
cargo check -p wasm --target wasm32-unknown-unknown
cargo clippy -p wasm --target wasm32-unknown-unknown
cd crates/wasm && NO_COLOR=false trunk build
```

Trunk needs `NO_COLOR=false` in shells that export `NO_COLOR=1`, because Trunk `0.21.14` expects a boolean value for its
`--no-color` option. The helper script sets this for `build-wasm`.

## Useful Groups

Before committing native-only changes:

```shell
scripts/dev.sh fmt-check
scripts/dev.sh check-native
scripts/dev.sh clippy-native
```

Before touching the web demo:

```shell
scripts/dev.sh fmt-check
scripts/dev.sh check-wasm
scripts/dev.sh clippy-wasm
scripts/dev.sh build-wasm
```

Before releasing/testing the plugin in a DAW:

```shell
scripts/dev.sh fmt-check
scripts/dev.sh clippy-plugin
scripts/dev.sh bundle-plugin
```
