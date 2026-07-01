# Rust Tooling And Tests

Detailed Rust workspace tooling instructions for agents. Start from `../AGENTS.md`; this file holds the longer rules so
the entrypoint stays small.

## Cargo And Formatting

- Run Cargo commands from the `rust/` directory unless a task explicitly targets the repository root.
- The Rust helper script is `rust/scripts/dev.sh`; repository docs that say `scripts/dev.sh ...` assume the command is
  being run from `rust/`. If that command appears missing from the repository root, use the documented `cd rust` context
  rather than adding a root wrapper.
- `rustfmt` is intentionally run with nightly.
- Keep `clippy::pedantic` enabled at workspace level.

## Lints And Idioms

- Do not add `#[allow(...)]` lint exceptions without explicit human confirmation.
- Treat Clippy warnings as work to fix. If a lint looks wrong or harmful, stop and explain the tradeoff before changing
  code.
- Do not blindly trust idioms that are common in training data, examples, or generic Rust advice. Check whether the
  pattern improves this repository's readability, tooling, and maintenance before using it.
- Prefer type-safe representations over sentinel values. If a value has an unset/error/special state, encode that state
  in the type.
- Avoid false reuse. If a new helper or protocol makes callers prove a variant is impossible with `unreachable!`,
  `debug_assert!`, or "cannot happen here" comments, split the type or move the shared representation later in the flow.
  If a caller already knows the only valid case, keep the direct code there instead of routing through a generic policy
  method.
- Before keeping a new abstraction, check that it models production code that exists now. Do not add variants, traits, or
  policy layers for possible future cases; add them when the future case exists.
- Do not introduce macros unless the user explicitly agrees first.
- Avoid macro calls in ordinary Rust code unless they remove meaningful boilerplate or encode an established local
  pattern. Even familiar macros must earn their use; prefer direct syntax when it is clearer for IDE navigation and human
  readers.

## Dependencies

- Before adding custom generic utility code, evaluate existing crates for the job. If a crate is close but awkward,
  document the mismatch.
- Avoid generic `utils` crates. Put reusable primitives in focused crates that match their dependency surface, such as
  synchronization primitives in `sync`.
- Keep dependency versions moving forward. Prefer updating or forking the dependency that pins an old version over
  patching an obsolete transitive crate.

## Test Layout

- Do not add inline `mod tests { ... }` test bodies to production source files.
- Unit test bodies belong in each crate's `tests/unit/*.rs` files, wired from the production module with the small hook:

  ```rust
  #[cfg(test)]
  #[path = "../tests/unit/<module>.rs"]
  mod tests;
  ```

- Unit test files use the same basename as their production module: `src/thing.rs` maps to `tests/unit/thing.rs`, not
  `tests/unit/thing_test.rs`.
- New integration tests belong under `tests/integration/`, not directly under `tests/`.
- This layout keeps RustRover filtering production files versus test files predictably and keeps GitHub/tree views easier
  to scan.

## Known Sandbox Issue

- Plugin task-executor tests bind localhost TCP sockets with `TcpListener`. In Codex sandboxed runs, Rust test commands
  that include `midi-bpm-detector-plugin` or the whole workspace predictably fail with `Operation not permitted` unless
  the command has elevated sandbox permissions. Request that permission up front for those test commands; do not run a
  sandboxed probe just to rediscover the known localhost-bind failure.
