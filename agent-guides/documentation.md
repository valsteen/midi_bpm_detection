# Repository Documentation Guidance

Detailed repository-level documentation routing and wording rules for agents. Start from `../AGENTS.md`; this file holds
the longer docs rules so the entrypoint stays small.

## Routing

- Use `../docs/architecture.md` for stable architecture narrative.
- Use `../docs/runtime-lifecycle.md` for bootstrap wiring, ownership boundaries, and runtime data-flow diagrams.
- Use `../docs/plugin-flow.md` for plugin realtime/audio callback details.
- Use `../docs/bitwig-tempo-bridge.md` for the narrow plugin-to-Bitwig-controller-extension tempo bridge.
- Use `../docs/development.md` for build, lint, format, package, install, and run commands.
- Use `../docs/native-midi-flow.md` for native MIDI and desktop flow details.
- Use `../docs/algorithm-archaeology.md` for algorithm history, interval-domain terminology, and histogram reasoning.
- Use `../docs/lint-exceptions.md` when reviewing or changing Rust `#[allow(...)]`, Kotlin suppressions, or Detekt ignores.
- Use `../docs/engineering-style.md` for durable cross-language implementation style.
- Use `../docs/refactoring-guide.md` for refactoring smells, strategies, and stop conditions.

## Writing Rules

- When touching confusing code, clarify terminology near the code or in the relevant docs.
- Keep comments concise: where data comes from, where it goes, what moment in the flow it belongs to, and why the
  boundary exists.
