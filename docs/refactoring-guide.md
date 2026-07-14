# Refactoring Guide

This guide records repository preferences for behavior-preserving cleanup and structural changes. Use it together with
`engineering-style.md`, [development commands](development.md), and [lint exceptions](lint-exceptions.md).

## Refactoring Goals

Refactoring should make the next change safer, easier to review, or easier to explain. Do not refactor only to make code
look rearranged.

Good refactoring usually does at least one of these:

- makes ownership or lifecycle boundaries visible;
- narrows a function or type to the state it actually needs;
- replaces an ambiguous primitive with a meaningful type;
- removes false reuse or a misleading abstraction;
- preserves behavior while reducing hidden coupling.

## Common Smells And Strategies

### Lint-Driven Churn

Treat lints as signals, not objectives. For `too_many_lines`, do not mechanically split a function into helpers that
still borrow all of the same state. First look for ownership boundaries, lifecycle phases, and narrow inputs.

Stop if the refactor only hides the same ambiguity behind new names.

### Broad `self`

A helper method that takes `&mut self` can still access every field, even if it only needs one or two. Prefer one of:

- a method on a narrower capability that owns the relevant state;
- a free function with an exact parameter list;
- a small type that owns lifecycle state for the whole runtime, not just a borrowed parameter bundle.

### Borrowed DTOs

Temporary structs that only bundle references are suspect. They can be useful during exploration, but before finishing,
ask whether the struct models a real production concept.

If it has no lifecycle and only one receiver, plain parameters may tell the truth better.

### Opaque Booleans

When a helper returns `bool`, check whether the call site reads clearly without remembering what `true` means.

- Use a boolean when the predicate is obvious from the function name, such as `is_open`.
- Use a small enum for workflow outcomes, such as `BpmPublication::Required`.
- If only one enum variant matters at the call site, use `if outcome == Variant` rather than a `match` with empty
  branches.

### Generic Names

Names like `shared`, `sync`, `manager`, `lane`, and `state` need justification.

- Replace `shared` with the actual owners or consumers.
- Replace `sync` with `origin`, `snapshot`, `apply`, or another term that matches the lifecycle moment.
- Replace `manager` with the resource or workflow being managed.
- Replace temporary labels like `lane` unless they map to a durable runtime responsibility.

### False Reuse

If callers must prove a variant cannot happen with `unreachable!`, `debug_assert!`, or "cannot happen here" comments,
the abstraction is probably too broad. Split the type or move the shared representation later in the flow.

## Refactor Checklist

Before editing:

- Identify the behavior contract to preserve.
- Identify the owner, producer, and consumer for each stateful path you touch.
- Check whether the change crosses runtime, thread, build-root, or public API boundaries.
- Pick focused tests or checks that prove the preserved behavior.

During editing:

- Keep unrelated cleanup out of the diff.
- Prefer names that explain the current design, not the old design.
- Avoid adding new abstractions until the current code proves they remove real complexity.
- Keep comments short and tied to data flow, lifecycle, or boundary decisions.

Before finishing:

- Re-read the diff for hidden broad access, empty enum branches, stale names, and newly generic helpers.
- Run the narrowest useful verification, then broaden when the blast radius warrants it.
- Update any durable docs or audit handoffs that would otherwise describe the old structure.
