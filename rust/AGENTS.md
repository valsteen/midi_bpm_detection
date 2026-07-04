# AGENTS.md

Rust workspace instructions for AI coding agents working under `rust/`.

## Read These First

- The repository-level `../AGENTS.md` still applies.
- Follow `../docs/engineering-style.md` for general implementation style.
- Follow `../docs/refactoring-guide.md` for refactors, lint-driven work, and structural cleanup.
- Follow `architecture.md` for human-facing Rust workspace architecture, crate grouping, and dependency direction.
- Follow `agent-guides/tooling.md` for Cargo, rustfmt, Clippy, lint, dependency, and test-layout rules.
- Follow `agent-guides/architecture.md` for Rust workspace boundaries, realtime constraints, and communication patterns.
- Follow `agent-guides/documentation.md` for Rust-facing documentation routing and wording rules.

## Hard Rules

- Make small, reviewable changes. Prefer commits that map to one clear design or behavior step.
- Discuss non-obvious design choices before implementing them.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.
- Run Cargo commands from this `rust/` directory unless a task explicitly targets the repository root.
- Do not add `#[allow(...)]` lint exceptions without explicit human confirmation.
- Treat Clippy warnings as work to fix. If a lint looks wrong or harmful, stop and explain the tradeoff before changing code.
- Do not introduce macros unless the user explicitly agrees first.
- Keep plugin and WASM behavior unchanged unless the task explicitly targets them.
- Keep realtime/audio-thread constraints explicit in code and docs.
- When changing behavior, verify with the narrowest useful command first, then broaden verification when the blast radius grows.
