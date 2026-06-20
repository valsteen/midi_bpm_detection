#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
    cat <<'EOF'
Usage: scripts/dev.sh <command>

Formatting:
  fmt             Run rustfmt with nightly-only options
  fmt-check       Check formatting with nightly rustfmt

Native macOS/dev checks:
  check-desktop   Check the desktop TUI/GUI app
  check-plugin    Check the CLAP/VST3 plugin crate
  check-reset     Check the macOS MIDI reset utility
  check-native    Check desktop, plugin, and reset crates
  clippy-desktop  Run clippy for the desktop TUI/GUI app
  clippy-plugin   Run clippy for the CLAP/VST3 plugin crate
  clippy-reset    Run clippy for the MIDI reset utility
  clippy-native   Run clippy for desktop, plugin, and reset crates

Run/build commands:
  run-desktop     Run the desktop TUI/GUI app with local dev config paths
  bundle-plugin   Bundle the CLAP/VST3 plugin under target/bundled

WASM commands:
  check-wasm      Check the wasm crate for wasm32-unknown-unknown
  clippy-wasm     Run clippy for the wasm crate
  build-wasm      Build the Trunk web app
EOF
}

run_desktop_env() {
    BPM_DETECTION_CONFIG="$ROOT/.data" \
    BPM_DETECTION_DATA="$ROOT/.data" \
    MIDI_TUI_CONFIG="$ROOT/.config" \
    MIDI_TUI_DATA="$ROOT/.data" \
    MIDI_TUI_LOG_LEVEL="${MIDI_TUI_LOG_LEVEL:-info}" \
    "$@"
}

command="${1:-}"

case "$command" in
    fmt)
        cargo +nightly fmt --all
        ;;
    fmt-check)
        cargo +nightly fmt --all -- --check
        ;;
    check-desktop)
        cargo check -p tui
        ;;
    check-plugin)
        cargo check -p midi-bpm-detector-plugin
        ;;
    check-reset)
        cargo check -p midi-reset
        ;;
    check-native)
        cargo check -p tui -p midi-bpm-detector-plugin -p midi-reset
        ;;
    clippy-desktop)
        cargo clippy -p tui --all-targets
        ;;
    clippy-plugin)
        cargo clippy -p midi-bpm-detector-plugin --all-targets
        ;;
    clippy-reset)
        cargo clippy -p midi-reset --all-targets
        ;;
    clippy-native)
        cargo clippy -p tui -p midi-bpm-detector-plugin -p midi-reset --all-targets
        ;;
    run-desktop)
        run_desktop_env cargo run -p tui --bin bpm_detector_tui
        ;;
    bundle-plugin)
        cargo xtask bundle midi-bpm-detector-plugin --release
        ;;
    check-wasm)
        cargo check -p wasm --target wasm32-unknown-unknown
        ;;
    clippy-wasm)
        cargo clippy -p wasm --target wasm32-unknown-unknown
        ;;
    build-wasm)
        (
            cd crates/wasm
            NO_COLOR=false trunk build
        )
        ;;
    "" | help | -h | --help)
        usage
        ;;
    *)
        echo "Unknown command: $command" >&2
        echo >&2
        usage >&2
        exit 2
        ;;
esac
