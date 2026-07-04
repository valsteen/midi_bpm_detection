# Repository Structure And Cross-Boundary Rules

Detailed repository shape and cross-build-root instructions for agents. Start from `../AGENTS.md`; this file holds the
longer routing and architecture rules so the entrypoint stays small.

## Monorepo Shape

- `rust/`: Cargo workspace for the BPM detector core, plugin, desktop app, WASM demo, shared GUI, MIDI service, Rust
  tools, and the foundation parameter stack. Crates are grouped by role under `rust/crates/foundation/`,
  `rust/crates/entrypoints/`, `rust/crates/bpm/`, `rust/crates/support/`, and `rust/crates/tools/`. Follow
  `../rust/AGENTS.md` for Rust-specific instructions.
- `extension/`: Gradle workspace for Bitwig controller extensions and reusable extension libraries. Follow
  `../extension/AGENTS.md` for Kotlin/Bitwig-specific instructions.
- `docs/`: public architecture, development, runtime-flow, and cross-build-root documentation.

Keep Rust and Kotlin as separate build roots. Do not create a root mega-build that makes Cargo own the Kotlin extension
or Gradle own the Rust workspace.

## Cross-Boundary Architecture

- The production Bitwig tempo-control path crosses both build roots: the Rust plugin estimates BPM, and the Kotlin Bitwig
  controller extension owns the Bitwig transport-tempo write.
- Use `../docs/bitwig-tempo-bridge.md` for the plugin-to-extension rendezvous and socket bridge.
- Keep the bridge narrow. Do not turn it into a general remote-control protocol unless a concrete feature needs that.
- Do not move Bitwig controller API dependencies into `rust/`.
- Do not move Rust plugin or egui dependencies into `extension/`.
- Rust product, domain, and application crates may depend down into `rust/crates/foundation/`; foundation crates must not
  depend back up into BPM-specific product crates.
