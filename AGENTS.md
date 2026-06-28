# AGENTS.md

Repository-level instructions for AI coding agents working on this monorepo.

## Working Style

- Make small, reviewable changes. Prefer commits that map to one clear design or behavior step.
- Discuss non-obvious design choices before implementing them.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.
- When changing behavior, verify with the narrowest useful command first, then broaden verification when the blast radius grows.

## Instruction And Command Consistency

- If an instruction, documented command, or workflow note does not work, first check whether the instruction is stale,
  incomplete, or being run from the wrong build root. Do not add compatibility code, wrappers, build plumbing, or product
  behavior just to make a stale instruction true.
- Prefer fixing the instruction or documentation when the code is already in the right shape. If changing code or tooling
  still seems like the right fix, make the reason explicit and ask the human before doing it unless the task already
  requested that exact tooling change.
- Keep agent-facing instructions current and concise. When adding a new rule, remove or update any nearby stale wording
  that would point agents in a different direction.
- When asking the human for direction, ask a concrete action question with the options or consequence named. Do not hand
  back vague uncertainty.

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

## Long-running audit/refactor workflow

For long-running architecture, migration, audit, or refactor work, this repository can use repo-scoped Codex skills.

Use `$repo-audit-coordinator` for planning, slicing, decision logging, and maintaining audit continuity.

Use `$bounded-implementer` for executing exactly one bounded implementation slice from a coordinator brief.

Keep active audit coordination state under the ignored local workspace `.codex/audits/<audit-name>/`.
This includes active slice briefs, fresh-context handovers, implementation back-handoffs, branch checkpoints, command
logs, and work-in-progress status.

Only promote long-lived project documentation or enduring AI instructions into tracked public docs. When an older branch
already has tracked transient audit artifacts, migrate them into `.codex/audits/<audit-name>/` before appending new
coordination state.

Do not rely on chat memory as the source of truth. Before assuming a change is local, check relevant component boundaries, shared contracts, generated code, build/test/CI paths, runtime configuration, and integration points.
