# AGENTS.md

Rust workspace instructions for AI coding agents working under `rust/`.

## Working Style

- Make small, reviewable changes. Prefer commits that map to one clear design or behavior step.
- Discuss non-obvious design choices before implementing them.
- Do not introduce macros unless the user explicitly agrees first.
- Avoid macro calls in ordinary Rust code unless they remove meaningful boilerplate or encode an established local
  pattern. Even familiar macros must earn their use; prefer direct syntax when it is clearer for IDE navigation and human
  readers.
- Do not add `#[allow(...)]` lint exceptions without explicit human confirmation.
- Treat clippy warnings as work to fix. If a lint looks wrong or harmful, stop and explain the tradeoff before changing code.
- When changing behavior, verify with the narrowest useful command first, then broaden verification when the blast radius grows.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.

## Rust And Tooling

- Run Cargo commands from this `rust/` directory unless a task explicitly targets the repository root.
- The Rust helper script is `rust/scripts/dev.sh`; repository docs that say `scripts/dev.sh ...` assume the command is
  being run from `rust/`. If that command appears missing from the repository root, use the documented `cd rust` context
  rather than adding a root wrapper.
- `rustfmt` is intentionally run with nightly.
- Keep `clippy::pedantic` enabled at workspace level.
- Do not blindly trust idioms that are common in training data, examples, or generic Rust advice. Check whether the
  pattern improves this repository's readability, tooling, and maintenance before using it.
- Prefer type-safe representations over sentinel values. If a value has an unset/error/special state, encode that state in the type.
- Avoid false reuse. If a new helper/protocol makes callers prove a variant is impossible with `unreachable!`, `debug_assert`, or "cannot happen here" comments, split the type or move the shared representation later in the flow. If a caller already knows the only valid case, keep the direct code there instead of routing through a generic policy method.
- Before keeping a new abstraction, check that it models production code that exists now. Do not add variants, traits, or policy layers for possible future cases; add them when the future case exists.
- Before adding custom generic utility code, evaluate existing crates for the job. If a crate is close but awkward, document the mismatch.
- Avoid generic `utils` crates. Put reusable primitives in focused crates that match their dependency surface, such as synchronization primitives in `sync`.
- Keep dependency versions moving forward. Prefer updating/forking the dependency that pins an old version over patching an obsolete transitive crate.

## Rust Test Layout

- Do not add inline `mod tests { ... }` test bodies to production source files.
- Unit test bodies belong in each crate's `tests/unit/*.rs` files, wired from the production module with the small hook:

  ```rust
  #[cfg(test)]
  #[path = "../tests/unit/<module>.rs"]
  mod tests;
  ```

- Unit test files use the same basename as their production module: `src/thing.rs` maps to `tests/unit/thing.rs`, not `tests/unit/thing_test.rs`.
- New integration tests belong under `tests/integration/`, not directly under `tests/`.
- This layout keeps RustRover filtering production files versus test files predictably and keeps GitHub/tree views easier to scan.

## Architecture Boundaries

- Preserve the separation between the production plugin, native desktop mode, WASM showcase mode, shared GUI, MIDI service, and BPM detection core.
- Split crates primarily by dependency surface, then refine by responsibility when a crate grows too broad.
- Do not move MIDI/native dependencies into `gui`.
- Do not move egui/UI dependencies into `bpm_detection_midi`.
- Keep plugin and WASM behavior unchanged unless the task explicitly targets them.
- Treat plugin mode as the production constraint. Desktop and WASM are useful for iteration, demos, and architecture validation.

## Realtime Constraints

- Keep realtime/audio-thread constraints explicit in code and docs.
- Avoid blocking locks, allocation, and blocking reads on audio-critical paths.
- Prefer batch/buffer-oriented processing for plugin MIDI/audio flow. Plugin MIDI events arrive with timing inside processing buffers; they are not an ordinary event stream.
- Be careful when introducing cross-thread communication. Document which side owns the thread, which side calls into it, and whether calls can fail or block.

## Communication Patterns

- The project is moving away from a broad central event bus where one enum/orchestrator knows everything.
- Prefer explicit producer/consumer or service-handle boundaries where components connect during bootstrap, then communicate directly through narrow surfaces.
- The closure-based service boundary is intentional: callers pass thread-safe closures to the owning service, and the service owns whatever channel/message ceremony is required internally.
- Re-evaluate these choices as the architecture evolves. Do not preserve a pattern just because it exists.

## Documentation

- When touching confusing code, clarify terminology near the code or in the relevant root docs.
- Keep comments concise: where data comes from, where it goes, what moment in the flow it belongs to, and why the boundary exists.
- When documenting refactors, do not describe unchanged behavior as "restored" or "now" happening. Say that the behavior is preserved and name only the structural change.
- When documenting refactor targets, include the non-goal or stop condition so future agents do not expand the note into speculative architecture work.
- Use `../docs/plugin-flow.md` for plugin realtime/audio callback details.
- Use `../docs/bitwig-tempo-bridge.md` for the narrow plugin-to-Bitwig-controller-extension tempo bridge.
- Use `../docs/development.md` for build, lint, format, and run commands.
- Use `../docs/native-midi-flow.md` for native MIDI and desktop flow details.
- Use `../docs/algorithm-archaeology.md` for algorithm history, interval-domain terminology, and histogram reasoning.
- Use `../docs/lint-exceptions.md` when reviewing or changing existing `#[allow(...)]` lint exceptions.

## Current Direction

- The `desktop` crate is the native app path; the old TUI-first native shell has been retired.
- The desktop crate should own native MIDI device selection and startup orchestration while reusing the shared `gui` crate.
