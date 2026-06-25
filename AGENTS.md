# AGENTS.md

Repository-level instructions for AI coding agents working on this monorepo.

## Working Style

- Make small, reviewable changes. Prefer commits that map to one clear design or behavior step.
- Discuss non-obvious design choices before implementing them.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.
- When changing behavior, verify with the narrowest useful command first, then broaden verification when the blast radius grows.

## Monorepo Shape

- `rust/`: Cargo workspace for the BPM detector core, plugin, desktop app, WASM demo, shared GUI, MIDI service, and Rust tools. Follow `rust/AGENTS.md` for Rust-specific instructions.
- `extension/`: Gradle workspace for Bitwig controller extensions and reusable extension libraries. Follow `extension/AGENTS.md` for Kotlin/Bitwig-specific instructions.
- `docs/`: public architecture, development, runtime-flow, and cross-build-root documentation.

Keep Rust and Kotlin as separate build roots. Do not create a root mega-build that makes Cargo own the Kotlin extension
or Gradle own the Rust workspace.

## Cross-Boundary Architecture

- The production Bitwig tempo-control path crosses both build roots: the Rust plugin estimates BPM, and the Kotlin
  Bitwig controller extension owns the Bitwig transport-tempo write.
- Use `docs/bitwig-tempo-bridge.md` for the plugin-to-extension rendezvous and socket bridge.
- Use `docs/handoff/bitwig-extension-rendezvous.md` when carrying the same rendezvous pattern into another project.
- Keep the bridge narrow. Do not turn it into a general remote-control protocol unless a concrete feature needs that.
- Do not move Bitwig controller API dependencies into `rust/`.
- Do not move Rust plugin or egui dependencies into `extension/`.

## Documentation Routing

- Use `docs/architecture.md` for stable architecture narrative.
- Use `docs/runtime-lifecycle.md` for bootstrap wiring, ownership boundaries, and runtime data-flow diagrams.
- Use `docs/plugin-flow.md` for plugin realtime/audio callback details.
- Use `docs/bitwig-tempo-bridge.md` for the narrow plugin-to-Bitwig-controller-extension tempo bridge.
- Use `docs/development.md` for build, lint, format, package, install, and run commands.
- Use `docs/native-midi-flow.md` for native MIDI and desktop flow details.
- Use `docs/algorithm-archaeology.md` for algorithm history, interval-domain terminology, and histogram reasoning.
- Use `docs/lint-exceptions.md` when reviewing or changing Rust `#[allow(...)]`, Kotlin suppressions, or Detekt ignores.

When touching confusing code, clarify terminology near the code or in the relevant docs. Keep comments concise: where
data comes from, where it goes, what moment in the flow it belongs to, and why the boundary exists.
