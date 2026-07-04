# Rust Documentation Guidance

Detailed Rust-facing documentation instructions for agents. Start from `../AGENTS.md`; this file holds the longer docs
rules so the entrypoint stays small.

## Writing Rules

- When touching confusing code, clarify terminology near the code or in the relevant root docs.
- Keep comments concise: where data comes from, where it goes, what moment in the flow it belongs to, and why the
  boundary exists.
- When documenting refactors, do not describe unchanged behavior as "restored" or "now" happening. Say that the behavior
  is preserved and name only the structural change.
- When documenting refactor targets, include the non-goal or stop condition so future agents do not expand the note into
  speculative architecture work.

## Routing

- Use `../architecture.md` for human-facing Rust workspace architecture, crate maps, crate groups, and Rust runtime
  constraints.
- Use `../../docs/plugin-flow.md` for plugin realtime/audio callback details.
- Use `../../docs/bitwig-tempo-bridge.md` for the narrow plugin-to-Bitwig-controller-extension tempo bridge.
- Use `../../docs/development.md` for build, lint, format, and run commands.
- Use `../../docs/native-midi-flow.md` for native MIDI and desktop flow details.
- Use `../../docs/algorithm-archaeology.md` for algorithm history, interval-domain terminology, and histogram reasoning.
- Use `../../docs/lint-exceptions.md` when reviewing or changing existing `#[allow(...)]` lint exceptions.
