# Rust Architecture Boundaries

Detailed Rust workspace architecture instructions for agents. Start from `../AGENTS.md`; this file holds the longer
boundary rules so the entrypoint stays small.

## Workspace Boundaries

- Preserve the separation between the production plugin, native desktop mode, WASM showcase mode, shared GUI, MIDI
  service, and BPM detection core.
- Split crates primarily by dependency surface, then refine by responsibility when a crate grows too broad.
- Keep reusable parameter metadata, optional reusable parameter value types, and optional plugin-host bridges under
  `crates/foundation/`.
- Product, domain, and application crates may depend down into foundation crates; foundation crates must not depend back
  up into BPM-specific product crates such as `midi-bpm-detector-plugin`, `bpm_detection_core`, `bpm_detection_midi`, or
  `gui`.
- Do not move MIDI/native dependencies into `gui`.
- Do not move egui/UI dependencies into `bpm_detection_midi`.
- Keep plugin and WASM behavior unchanged unless the task explicitly targets them.
- Treat plugin mode as the production constraint. Desktop and WASM are useful for iteration, demos, and architecture
  validation.

## Realtime Constraints

- Keep realtime/audio-thread constraints explicit in code and docs.
- Avoid blocking locks, allocation, and blocking reads on audio-critical paths.
- Prefer batch/buffer-oriented processing for plugin MIDI/audio flow. Plugin MIDI events arrive with timing inside
  processing buffers; they are not an ordinary event stream.
- Be careful when introducing cross-thread communication. Document which side owns the thread, which side calls into it,
  and whether calls can fail or block.
- Fixed-capacity buffers in plugin/core runtime paths are intentional. If a test overflows its stack while constructing
  those structures, fix the test harness stack or test shape; do not replace production storage with heap-backed
  collections.

## Communication Patterns

- The project is moving away from a broad central event bus where one enum or orchestrator knows everything.
- Prefer explicit producer/consumer or service-handle boundaries where components connect during bootstrap, then
  communicate directly through narrow surfaces.
- The closure-based service boundary is intentional: callers pass thread-safe closures to the owning service, and the
  service owns whatever channel/message ceremony is required internally.
- Re-evaluate these choices as the architecture evolves. Do not preserve a pattern just because it exists.
