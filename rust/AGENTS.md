# AGENTS.md

Rust workspace instructions for AI coding agents working under `rust/`.

## Read These First

- The repository-level `../AGENTS.md` still applies.
- Use `../docs/README.md` to select task-specific public documentation.
- Follow `architecture.md` for human-facing Rust workspace architecture, crate grouping, and dependency direction.
- Follow `agent-guides/tooling.md` for Cargo, rustfmt, Clippy, lint, dependency, and test-layout rules.
- Follow `agent-guides/architecture.md` for Rust workspace boundaries, realtime constraints, and communication patterns.
- Follow `agent-guides/documentation.md` for the Rust-specific consequences of the repository documentation policy.

## Hard Rules

- Run Cargo commands from this `rust/` directory unless a task explicitly targets the repository root.
- Keep plugin and WASM behavior unchanged unless the task explicitly targets them.
- Keep realtime/audio-thread constraints explicit in code and docs.
- Treat plugin mode as the production constraint when a shared design must satisfy different runtime modes.
- Keep Rust-specific lint, macro, dependency, and test-layout policy in `agent-guides/tooling.md`.
