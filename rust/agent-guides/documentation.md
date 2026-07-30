# Rust Documentation Guidance

Detailed Rust-facing documentation instructions for agents. Start from `../AGENTS.md`; this file holds the longer docs
rules so the entrypoint stays small.

The repository-level [documentation guidance](../../agent-guides/documentation.md) owns canonical placement, public
versus private state, present-tense wording, and document growth. Use the
[documentation index](../../docs/README.md) to select public documentation.

## Rust-Specific Ownership

- Use `../architecture.md` for crate maps, crate groups, Rust dependency direction, parameter-stack ownership, and
  constraints shared by multiple Rust entrypoints.
- Keep plugin callback sequencing, desktop MIDI ownership, and WASM scheduling in the task-specific runtime document
  selected through the root index rather than duplicating those flows in the crate map.
- In realtime comments, name the constrained thread or callback, the crossing destination, and whether the handoff may
  allocate, block, fail, coalesce, or lose intermediate values.
- Keep algorithm and configuration terminology consistent with the canonical domain document; do not invent a
  runtime-local synonym for the same concept.
