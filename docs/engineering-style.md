# Engineering Style

This guide records durable implementation preferences for this repository. It applies to new features, bug fixes,
refactors, reviews, and documentation work across build roots.

## Core Principles

- Prefer ownership clarity over surface brevity. A shorter function is not an improvement if it hides who owns state or
  which subsystem is allowed to touch it.
- Keep dependencies exact. Functions, methods, and constructors should receive the state they need, not a larger object
  that happens to contain it.
- Name values by their lifecycle, source, destination, or counterparties. Avoid generic names like `shared`, `sync`,
  `manager`, `lane`, or `state` when the code can name who shares it, who synchronizes it, or what phase it represents.
- Model meaningful states with types when a boolean would make callers remember what `true` means. When only one enum
  variant matters at a call site, use a plain `if` against the named variant rather than an empty-branch `match`.
- Use methods when the behavior belongs to the receiver's own state. Use free functions for cross-capability workflows
  when an exact parameter list makes the dependencies clearer.
- Prefer explicit producer/consumer boundaries over broad orchestration objects that can reach everything.
- Preserve runtime contracts deliberately. Make behavior changes explicit; for structural work, say which behavior is
  preserved.
- Avoid speculative abstraction. Add variants, traits, policy layers, and reusable helpers when current production code
  needs them, not because a future shape is imaginable.
- Keep comments concise and local. Explain where data comes from, where it goes, what lifecycle moment it belongs to, or
  why a boundary exists.
- Keep tests proportional to blast radius. Narrow behavior-preserving changes may only need focused existing tests;
  shared contracts, runtime paths, or user-visible behavior need broader verification.

## Naming

Names should make responsibility legible without requiring the reader to reconstruct the whole call graph.

- Name shared state by its owners or readers, such as `gui_task_config`, rather than just `shared_config`.
- Distinguish handoff or mailbox state from retained live state, such as `remote_handoff` versus `live_remote`.
- Name source-like values as origins, inputs, or producers. Name action-like values as commands or effects.
- Rename stale terms when the shape changes. Do not preserve a name that describes a previous design.

## Boundaries

When adding or changing a boundary, check what the caller can now access.

- A helper that takes all of a broad object still lets hidden coupling grow inside the helper.
- A long-lived struct should model lifecycle ownership. A temporary borrowed struct is only useful when it clarifies a
  real concept, not when it is a disguised parameter bundle.
- A capability type should expose methods only for behavior that genuinely belongs to that capability's state.
- Cross-capability operations should make their dependencies visible at the call site.

## Documentation

Documentation should separate current behavior from future ideas.

- Describe stable architecture in the relevant public docs.
- When documenting refactors, avoid implying preserved behavior was newly added. Say the behavior is preserved and name
  the structural change.
