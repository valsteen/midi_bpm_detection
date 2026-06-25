#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WASM_BINDGEN_CLI_VERSION="0.2.125"
WASM_PORT="${WASM_PORT:-8080}"
WASM_DEV_URL="http://127.0.0.1:${WASM_PORT}/midi_bpm_detection/#dev"

usage() {
    cat <<'EOF'
Usage: scripts/dev.sh <command>

Setup:
  doctor          Check local tool setup
  doctor-wasm     Check local WASM tool setup

Formatting:
  fmt             Run rustfmt with nightly-only options
  fmt-check       Check formatting with nightly rustfmt

Native macOS/dev checks:
  check-desktop   Check the native desktop GUI app
  check-plugin    Check the CLAP/VST3 plugin crate
  check-reset     Check the macOS MIDI reset utility
  check-native    Check desktop, plugin, and reset crates
  test-core        Test the core BPM detection crate
  test-desktop     Test the native desktop GUI app
  test-native      Test core and desktop crates
  clippy-desktop  Run clippy for the native desktop GUI app
  clippy-plugin   Run clippy for the CLAP/VST3 plugin crate
  clippy-reset    Run clippy for the MIDI reset utility
  clippy-native   Run clippy for desktop, plugin, and reset crates
  clippy-all      Run clippy for current native and WASM build modes
  verify-native   Run the usual native pre-commit checks

Run/build commands:
  run-desktop     Run the native desktop GUI app with local dev config paths
  bundle-plugin   Bundle the CLAP/VST3 plugin under target/bundled
  verify-plugin   Run the usual plugin pre-DAW checks

WASM commands:
  check-wasm      Check the wasm crate for wasm32-unknown-unknown
  test-wasm       Test the wasm crate with wasm-bindgen-test-runner
  clippy-wasm     Run clippy for the wasm crate
  build-wasm      Build the Trunk web app
  serve-wasm      Serve the Trunk web app for browser testing
  verify-wasm     Run the usual wasm build/lint checks
EOF
}

require_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        echo "$2" >&2
        return 1
    fi
}

doctor_wasm() {
    local failed=0

    require_command trunk "Install with: cargo install trunk" || failed=1
    require_command wasm-bindgen "Install with: cargo install -f wasm-bindgen-cli --version $WASM_BINDGEN_CLI_VERSION" || failed=1
    require_command wasm-bindgen-test-runner "Install with: cargo install -f wasm-bindgen-cli --version $WASM_BINDGEN_CLI_VERSION" || failed=1

    if ! rustup target list --installed | grep -qx "wasm32-unknown-unknown"; then
        echo "Missing Rust target: wasm32-unknown-unknown" >&2
        echo "Install with: rustup target add wasm32-unknown-unknown" >&2
        failed=1
    fi

    if command -v wasm-bindgen >/dev/null 2>&1; then
        local installed_wasm_bindgen
        installed_wasm_bindgen="$(wasm-bindgen --version | awk '{print $2}')"
        if [[ "$installed_wasm_bindgen" != "$WASM_BINDGEN_CLI_VERSION" ]]; then
            echo "wasm-bindgen-cli version mismatch: found $installed_wasm_bindgen, expected $WASM_BINDGEN_CLI_VERSION" >&2
            echo "Install with: cargo install -f wasm-bindgen-cli --version $WASM_BINDGEN_CLI_VERSION" >&2
            failed=1
        fi
    fi

    if [[ "$failed" -eq 0 ]]; then
        echo "WASM tool setup looks good."
    fi

    return "$failed"
}

run_desktop_env() {
    BPM_DETECTION_CONFIG="$ROOT/.data" \
    BPM_DETECTION_DATA="$ROOT/.data" \
    "$@"
}

command="${1:-}"

case "$command" in
    doctor)
        require_command cargo "Install Rust with: https://rustup.rs" || exit 1
        require_command rustup "Install Rust with: https://rustup.rs" || exit 1
        require_command cargo-clippy "Install with: rustup component add clippy" || exit 1
        cargo +nightly fmt --version >/dev/null
        doctor_wasm
        ;;
    doctor-wasm)
        doctor_wasm
        ;;
    fmt)
        cargo +nightly fmt --all
        ;;
    fmt-check)
        cargo +nightly fmt --all -- --check
        ;;
    check-desktop)
        cargo check -p desktop
        ;;
    check-plugin)
        cargo check -p midi-bpm-detector-plugin
        ;;
    check-reset)
        cargo check -p midi-reset
        ;;
    check-native)
        cargo check -p desktop -p midi-bpm-detector-plugin -p midi-reset
        ;;
    test-core)
        cargo test -p bpm_detection_core
        ;;
    test-desktop)
        cargo test -p desktop
        ;;
    test-native)
        cargo test -p bpm_detection_core -p desktop
        ;;
    clippy-desktop)
        cargo clippy -p desktop --all-targets
        ;;
    clippy-plugin)
        cargo clippy -p midi-bpm-detector-plugin --all-targets
        ;;
    clippy-reset)
        cargo clippy -p midi-reset --all-targets
        ;;
    clippy-native)
        cargo clippy -p desktop -p midi-bpm-detector-plugin -p midi-reset --all-targets
        ;;
    clippy-all)
        "$0" clippy-native
        "$0" clippy-wasm
        ;;
    verify-native)
        "$0" fmt-check
        "$0" test-native
        "$0" check-native
        "$0" clippy-native
        ;;
    run-desktop)
        run_desktop_env cargo run -p desktop --bin desktop
        ;;
    bundle-plugin)
        cargo xtask bundle midi-bpm-detector-plugin --release
        ;;
    verify-plugin)
        "$0" fmt-check
        "$0" clippy-plugin
        "$0" bundle-plugin
        ;;
    check-wasm)
        cargo check -p wasm --target wasm32-unknown-unknown
        ;;
    test-wasm)
        cargo test -p wasm --target wasm32-unknown-unknown
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
    serve-wasm)
        echo "Open: $WASM_DEV_URL"
        (
            cd crates/wasm
            NO_COLOR=false trunk serve --port "$WASM_PORT" --open false
        )
        ;;
    verify-wasm)
        "$0" doctor-wasm
        "$0" fmt-check
        "$0" check-wasm
        "$0" test-wasm
        "$0" clippy-wasm
        "$0" build-wasm
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
