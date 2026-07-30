# Engineering Style

This guide records durable implementation preferences for this repository. It applies to new features, bug fixes,
refactors, reviews, and documentation work across build roots.

## Core Principles

- Prefer ownership clarity over surface brevity. A shorter function is not an improvement if it hides who owns state or
  which subsystem may touch it.
- Keep dependencies exact. Functions, methods, and constructors receive the state or capability they need, not a larger
  object that happens to contain it.
- Name values by lifecycle, source, destination, or counterparties. Avoid generic names such as `shared`, `sync`,
  `manager`, `lane`, or `state` when the code can name who shares it, what is synchronized, or which phase it represents.
- Model meaningful states with types when a boolean would make callers remember what `true` means.
- Use methods when behavior belongs to the receiver's own state. Use free functions for cross-capability workflows when
  an exact parameter list makes dependencies clearer.
- Make producers, consumers, and runtime crossings explicit. Prefer narrow peer or service boundaries over an object,
  dispatcher, or event enum that can reach every subsystem.
- Preserve runtime contracts deliberately. Make behavior changes explicit; for structural work, state which observable
  behavior remains unchanged.
- Avoid speculative abstraction. Add variants, traits, policy layers, reusable helpers, and indirection when current
  production code needs them, not because a future shape is imaginable.
- Keep ordinary code language-native, searchable, and debuggable. Do not trade visible dependencies for custom
  vocabulary, dynamic dispatch, allocation, or generated ceremony without a concrete benefit.
- Keep comments concise and local. Explain data origin, destination, lifecycle moment, or the reason a boundary exists.
- Keep verification proportional to blast radius. Start with focused evidence, then broaden when shared contracts,
  runtime paths, or user-visible behavior can be affected.

## Thin End-To-End Slices

When a design is uncertain, establish the smallest real loop that crosses the relevant production boundaries. A useful
slice reaches a real consumer or observable output; a new abstraction exercised only by its own tests is not end-to-end
evidence.

- Observe the real path at the earliest safe point. Prefer consumer-visible behavior or runtime evidence over repeated
  reviews of an unexercised abstraction.
- Settle the slice's ownership, scheduling, naming, absence/error behavior, dependency direction, module placement, and
  developer ergonomics before widening it.
- Keep semantic symmetry across operating modes: express the same domain operation consistently while allowing plugin,
  desktop, and WASM to use different mechanisms where their lifecycles or constraints differ.
- Do not build a framework for the slice's imagined siblings. A second completed use can justify extracting shared
  machinery; it does not require extraction when direct code remains clearer.
- Keep diagnostic scaffolding minimal and outside the durable product surface unless it becomes a real supported
  capability.

## Shared State And Consistency

Before choosing atomics, locks, channels, callbacks, snapshots, or task messages, identify the logical state and its
invariants.

- Values that must describe one coherent configuration belong to one logical update. Readers should not observe a mixed
  epoch merely because each field can be updated independently.
- Separate atomics are appropriate only when each value is independently meaningful, mixed observations are acceptable,
  and the required memory ordering is understood.
- Name who publishes the state, who observes it, and which side owns validation and normalization.
- Distinguish a latest-value snapshot from an ordered event or cumulative delta. Each has different loss, ordering, and
  backpressure semantics.
- Prefer replacement snapshots for configuration when consumers need the latest coherent configuration rather than
  every intermediate edit.
- Use the simplest mechanism that preserves the required semantics and the constraints of the boundary. Visual
  consistency between runtime modes is not a reason to impose one synchronization primitive on all of them.

## Naming

Names should make responsibility legible without reconstructing the entire call graph.

- Name shared state by its owners or readers, such as `gui_task_config`, rather than just `shared_config`.
- Distinguish handoff or mailbox state from retained live state, such as `remote_handoff` versus `live_remote`.
- Name source-like values as origins, inputs, snapshots, or producers. Name action-like values as commands, requests,
  publications, or effects.
- Use plain domain verbs and nouns. A generic architectural term is useful only when it makes the concrete capability
  easier—not harder—to discover.
- Rename stale terms when the shape changes. Do not preserve a name that describes an earlier design.
- Do not name a type after one call path when the type represents a durable domain concept used elsewhere.
- Treat genericity as a contract. A type named `Config`, `Manager`, or `Event` should not silently mean one specific
  runtime's configuration, owner, or message.
- Add qualifiers only for real ambiguity. Repeating module or type context in every name makes the important distinction
  harder to see.

## Boundaries And Adapters

When adding or changing a boundary, inspect what the caller can access and what dependencies the callee acquires.

- A helper that takes all of a broad object still lets hidden coupling grow inside the helper.
- A long-lived struct models lifecycle ownership. A temporary borrowed struct is useful only when it clarifies a real
  production concept, not when it disguises a parameter bundle.
- A capability type exposes behavior that genuinely belongs to its state. Cross-capability operations make their exact
  dependencies visible at the call site.
- Bootstrap advertises stable capabilities and wires known producers to known consumers. Prefer static wiring for
  stable relationships; do not add dynamic lookup or subscription merely to make components appear decoupled.
- A trait or interface should state one real consumer or producer capability. Do not combine unrelated capabilities to
  recreate a universal application surface behind a different name.
- Keep external API mechanics at the adapter edge. Product targeting, delivery, sequencing, and UI policy belong above
  host, MIDI, browser, socket, or framework adapters.
- Do not create one universal host/runtime wrapper. Several focused adapters may share a proven primitive while retaining
  distinct dependency, lifecycle, and failure contracts.
- Own a typed payload once. Introduce a projection only when it changes semantics, dependencies, lifecycle, or error
  handling—not merely to rename every field at another layer.

## Runtime Crossings

- Use explicit messages or snapshots when crossing ownership, thread, async-task, process, or realtime boundaries.
- Keep the constrained side's guarantees visible: plugin audio paths must not acquire blocking work, unbounded work,
  allocation, or unpredictable dispatch through a convenience abstraction.
- A boundary has a named owner for each direction. Do not allow multiple readers or writers to compete for the same
  state or stream unless the synchronization contract explicitly supports it.
- Decide where overload is dropped, coalesced, delayed, or rejected. Backpressure is part of the boundary contract, not
  a cleanup detail.
- Direct calls are appropriate when ownership and scheduling allow them. Channels and atomics are mechanisms for actual
  crossings, not proof that two components are decoupled.

## Production And Test Surfaces

- Do not add production-only-for-test constructors, getters, setters, branches, or protocol variants.
- Dependency-injection seams are appropriate when they model a real production boundary and allow its collaborator to be
  controlled in tests.
- Test-only sample values, fake effects, harness APIs, assertion helpers, and convenience wrappers belong in test
  sources.
- When reviewing a refactor, scan production declarations against production callers. If only tests use a declaration,
  remove it, move it to test code, or reshape the test around real production behavior.

## Tests

- Test observable behavior and stable consumer contracts, not declarations repeated as assertions. Do not instantiate a
  type and assert that getters return the literals used to construct it, or copy built-in defaults and inventories into
  tests merely to restate transient source data.
- Exact-value assertions are appropriate when the value is itself a durable external contract, such as a serialized key,
  socket frame, service-loader entry, or host parameter identifier, or when the test exercises a transformation that
  produces it. Make that consumer or transformation visible in the test setup and name.
- Prefer tests of state transitions, validation, parsing, effects, wiring, and failure behavior. If a data-only change
  breaks a test without changing a consumer-visible contract, the test is probably tautological.
