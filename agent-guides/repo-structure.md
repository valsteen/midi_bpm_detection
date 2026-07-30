# Repository Structure And Cross-Boundary Rules

Detailed repository shape and cross-build-root instructions for agents. Start from `../AGENTS.md`; this file holds the
longer routing and architecture rules so the entrypoint stays small.

## Monorepo Shape

- `rust/`: Cargo workspace for the BPM detector domain/model crates, plugin, desktop app, WASM demo, BPM shared GUI,
  native MIDI service, Rust tools, and the foundation parameter stack. Follow `../rust/AGENTS.md` for Rust-specific
  instructions and
  `../rust/architecture.md` for Rust crate grouping and dependency direction.
- `extension/`: Gradle workspace for Bitwig controller extensions and reusable extension libraries. Follow
  `../extension/AGENTS.md` for Kotlin/Bitwig-specific instructions.
- `docs/`: public architecture, development, runtime-flow, and cross-build-root documentation.

Keep Rust and Kotlin as separate build roots. Do not create a root mega-build that makes Cargo own the Kotlin extension
or Gradle own the Rust workspace.

## Agent Guide Shape

- Root `docs/` and `agent-guides/` files own cross-language principles, repository routing, and cross-build-root rules.
- Build-root `AGENTS.md` files select local guidance and state only the consequences specific to that build root.
- Build-root tooling guides own commands, configured tools, dependency policy, packaging, and test layout.
- Build-root architecture guides own language/runtime constraints and dependency direction.
- When a principle applies to both Rust and Kotlin, keep the shared form in `../docs/engineering-style.md`. Repeat it
  locally only when language semantics, tooling, or runtime shape changes how it must be applied.

## Cross-Boundary Architecture

- The production Bitwig tempo-control path crosses both build roots: the Rust plugin estimates BPM, and the Kotlin Bitwig
  controller extension owns the Bitwig transport-tempo write.
- Use `../docs/bitwig-tempo-bridge.md` for the plugin-to-extension rendezvous and socket bridge.
- Keep the bridge narrow. Do not turn it into a general remote-control protocol unless a concrete feature needs that.
- Do not move Bitwig controller API dependencies into `rust/`.
- Do not move Rust plugin or egui dependencies into `extension/`.
- Rust crate dependency direction is documented in `../rust/architecture.md`; keep that policy with the Rust build root
  instead of duplicating crate-level inventories here.

## Capability Grouping

- Group code by a stable product or runtime responsibility, then let dependency surface and lifecycle determine the
  physical crate, module, or Gradle boundary.
- Keep bootstrap and lifecycle composition outside capability implementations. Bootstrap should make stable producers,
  consumers, and their connections visible without becoming a service locator or catch-all orchestrator.
- Create a crate, Gradle module, or reusable package only for an independent dependency surface, lifecycle, or proven
  reuse boundary.
- Do not create a speculative parent abstraction or universal wrapper solely to make different runtime modes look
  symmetrical.
- Align vocabulary across plugin, desktop, WASM, and extension boundaries when the concepts are the same. Do not force
  identical directory trees or APIs when their dependencies and lifecycles differ.
