# Repository Documentation Guidance

Detailed repository-level documentation routing and wording rules for agents. Start from `../AGENTS.md`; this file holds
the longer docs rules so the entrypoint stays small.

## Routing And Retrieval

- Use the [documentation index](../docs/README.md) as the canonical route to public documentation.
- Keep retrieval shallow: the index selects a direct document or a stable subject hub, and a hub selects at most one
  bounded detail page for the concern in scope.
- Search headings and compact link descriptions before opening another page.
- Follow another document only when the selected page names it as a prerequisite or directly relevant detail.
- Do not duplicate the task-routing table in agent guides, build-root guides, or the root README.

## Canonical Ownership

- Give each durable fact one canonical home. Link to that home instead of copying its explanation elsewhere.
- Keep overview documents focused on stable relationships and routes. Put detailed commands, runtime sequences, wire
  formats, and algorithm rationale in the document that owns that concern.
- Preserve an established public path when splitting a subject so existing routes remain stable.
- Update the nearest hub and the [documentation index](../docs/README.md) when adding, moving, or removing a public
  document.
- Keep the graph at index-to-document or index-to-hub-to-detail depth. Do not build recursive indexes.

## Placing Or Growing Documentation

1. Name the durable fact and the concrete reader task.
2. Use the [documentation index](../docs/README.md) to identify its canonical owner.
3. Extend that document only when the new material answers the same reader question and has the same lifecycle.
4. Add a bounded detail page when the concern is independently selectable and would otherwise make readers load
   unrelated material.
5. Create a stable subject hub only when at least two bounded concerns share a durable scope.
6. Around 200 lines or 16 KB, review cohesion rather than splitting automatically.
7. Treat a page as a catchall when it cannot state one scope boundary or mixes concerns, evidence types, lifecycles, or
   audiences that readers would not normally need together.
8. Update the nearest route and verify changed links after reorganizing.

## Public And Private Documentation

- Keep public documentation stable, contributor-facing, and safe to publish.
- Keep plans, design exploration, audit handoffs, command logs, review packages, and work-in-progress status under
  ignored `.codex/audits/<audit-name>/` paths.
- Do not make private coordination state a prerequisite for understanding, building, testing, or contributing to the
  tracked project.
- Distill accepted decisions into the public architecture, runtime, development, algorithm, or engineering document
  that owns the resulting fact.
- Public docs describe current behavior and intended architecture. They are not a timeline of how a refactor or
  investigation arrived there.

## Writing Rules

- When touching confusing code, clarify terminology near the code or in the relevant docs.
- Keep comments concise: where data comes from, where it goes, what moment in the flow it belongs to, and why the
  boundary exists.
- Do not describe unchanged behavior as newly added, restored, or inherited from another implementation. State the
  current behavior directly and name only the structural distinction that readers need.
- Durable algorithm rationale may explain why the current model works as it does. Do not turn refactor chronology,
  migration history, or source-project provenance into architecture authority.
- When documenting a possible refactor, state the concrete tension and its stop condition so the note cannot silently
  grow into speculative architecture work.
