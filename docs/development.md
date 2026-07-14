# Development Commands

This project has one Rust workspace and one Kotlin/Gradle Bitwig extension workspace.

The Rust side has three main build modes:

- `desktop`: the native desktop GUI application in `rust/crates/entrypoints/desktop`, sharing the `gui` crate.
- `plugin`: the CLAP/VST3 plugin in `rust/crates/entrypoints/midi-bpm-detector-plugin`.
- `wasm`: the browser demo in `rust/crates/entrypoints/wasm`.

The Rust workspace lives under `rust/`. Unless a command says otherwise, run the Rust commands in this document from
that directory:

```shell
cd rust
```

The Rust workspace default members are `desktop` and `midi-reset`. The plugin and wasm crates should be checked
explicitly.

The Bitwig controller extension workspace lives under `extension/`. Run extension commands from that directory:

```shell
cd extension
```

The extension build uses the Gradle wrapper. Gradle needs a JDK to run, and the Kotlin build targets JVM 17. Toolchain
auto-download is enabled, so Gradle can provision JDK 17 when needed. Check what Gradle sees with:

```shell
./gradlew --version
./gradlew javaToolchains
```

## One Command Surface

From the `rust/` build root, use the helper script when you do not remember the exact Cargo invocation:

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

## Native Desktop

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
cargo check -p desktop
cargo test -p desktop
BPM_DETECTION_CONFIG=.data BPM_DETECTION_DATA=.data cargo run -p desktop --bin desktop
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

Bundled plugin artifacts are written under `rust/target/bundled` when viewed from the repository root.

## Bitwig Controller Extension

The companion Bitwig controller extension is a Gradle multi-project build under `extension/`.

Useful commands:

```shell
./gradlew test
./gradlew spotlessCheck detekt
./gradlew packageBitwigExtension
./gradlew printBitwigExtensionInstallDirectory
./gradlew installBitwigExtension
```

`packageBitwigExtension` produces:

```text
extension/extensions/beat-detection-controller/build/bitwig-extension/BeatDetectionExtension.bwextension
```

`installBitwigExtension` resolves the local Bitwig extension directory in this order:

1. `-PbitwigExtensionsDir=...`
2. `BITWIG_EXTENSIONS_DIR`
3. ignored `extension/gradle-local.properties`
4. `${user.home}/Documents/Bitwig Studio/Extensions`

To use a local file, copy `extension/gradle-local.properties.example` to `extension/gradle-local.properties` and set:

```properties
bitwigExtensionsDir=/Users/you/Documents/Bitwig Studio/Extensions
```

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
cargo clippy -p desktop -p midi-bpm-detector-plugin -p midi-reset --all-targets
cargo clippy -p wasm --target wasm32-unknown-unknown
```

`clippy-all` runs both `clippy-native` and `clippy-wasm`. Use it when you want to lint the current native desktop,
plugin, reset, and WASM build modes from one command.

The workspace enables `clippy::pedantic` as warnings. Treat Clippy warnings as issues to fix by default. Do not add a
new `#[allow(...)]` without human confirmation. If a lint is confirmed to be inappropriate, keep the allow narrow and add
a short reason near the affected code instead of disabling the lint broadly.

Existing lint exceptions are tracked in [lint exceptions](lint-exceptions.md). Treat that file as the current review
baseline, not as permission to add more exceptions silently.

Add `-- -D warnings` manually when you want CI-style strictness.

## Design Change Flow

Before introducing a new crate, macro, generic abstraction, or synchronization primitive, first compare the local design
against existing crates. The review should cover API fit, dependency surface, maturity, target/build impact, and whether
the external crate preserves the domain invariants this project needs.

Prefer integrating an existing crate when it gives a clear, debuggable surface. If a crate is close but awkward, document
the mismatch and consider opening an upstream issue before choosing a local implementation. Do not add new macros without
explicit design approval. When a macro is approved, keep its call site Rust-shaped: prefer attributes or derives on
ordinary Rust items, keep field names and types in Rust syntax, and avoid custom field-list DSLs for substantial item
definitions.

When touching type definitions or helper wrappers, check whether their dependency surface belongs in a more focused crate
or module. Avoid accumulating generic utilities inside feature-driven crates; keep a helper local only when it is
specialized to that feature's lifecycle or domain.

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
scripts/dev.sh verify-wasm-pages-dist
scripts/dev.sh publish-wasm-pages
```

Equivalent commands:

```shell
cargo check -p wasm --target wasm32-unknown-unknown
cargo test -p wasm --target wasm32-unknown-unknown
cargo clippy -p wasm --target wasm32-unknown-unknown
cd crates/entrypoints/wasm && NO_COLOR=false trunk build
cd crates/entrypoints/wasm && NO_COLOR=false trunk serve --port 8080 --open false
```

Trunk needs `NO_COLOR=false` in shells that export `NO_COLOR=1`, because Trunk `0.21.14` expects a boolean value for its
`--no-color` option. The helper script sets this for the Trunk commands.

`build-wasm` removes the previous generated `dist/` directory before running Trunk, then checks that the generated
`index.html` and service worker only reference files that exist in `dist/`. `verify-wasm-pages-dist` runs that generated
asset consistency check without rebuilding.

`verify-wasm` runs `doctor-wasm`, `fmt-check`, `check-wasm`, `test-wasm`, `clippy-wasm`, and `build-wasm`.

### GitHub Pages Publish

The browser demo is published from the `gh-pages` branch root. To verify, rebuild, commit, and push a new Pages build:

```shell
scripts/dev.sh publish-wasm-pages
```

The command refuses to publish from a dirty source tree by default, runs `verify-wasm`, rechecks the generated Pages
assets, copies `crates/entrypoints/wasm/dist/` into a temporary `gh-pages` worktree, commits the generated static files as
`build from <source-sha>`, and pushes `HEAD:gh-pages` to the `upstream` remote. Set `ALLOW_DIRTY_WASM_PUBLISH=1` only
when you intentionally want to publish an uncommitted local build.

After the push, GitHub will show a `Pages build and deployment` Actions run. That run is GitHub Pages deploying the
already-built files from `gh-pages`; this repository's CI workflow only validates the WASM app with `trunk build`.

### Browser Check

For the local browser loop:

```shell
scripts/dev.sh serve-wasm
```

Then open:

```text
http://127.0.0.1:8080/midi_bpm_detection/#dev
```

The path comes from `crates/entrypoints/wasm/Trunk.toml`, which sets `public_url = "/midi_bpm_detection/"`. The `#dev`
suffix matters during local development because `crates/entrypoints/wasm/index.html` skips service-worker registration
when the hash is `#dev`.
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
