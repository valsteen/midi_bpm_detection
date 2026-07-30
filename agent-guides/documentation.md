# Repository Documentation Guidance

Detailed repository-level documentation routing and wording rules for agents. Start from `../AGENTS.md`; this file holds
the longer docs rules so the entrypoint stays small.

## Audience And Routing

- The `Documentation` section of the root [README](../README.md) is the catalog of human-facing project documentation.
- Root and build-root `AGENTS.md` files route AI agents to the applicable policy under `agent-guides/`.
- Human-facing documents do not link into `agent-guides/`. Agent guidance may reference human-facing project documents
  when it needs the fact they own.
- Keep retrieval shallow: select a direct document or a stable subject hub, then at most one bounded detail page for the
  concern in scope.
- Search headings and compact link descriptions before opening another page.
- Follow another document only when the selected page names it as a prerequisite or directly relevant detail.
- Do not copy a human-facing explanation into agent policy merely to keep all context in one file.

## Canonical Ownership

- Give each durable fact one canonical home. Link to that home instead of copying its explanation elsewhere.
- Keep overview documents focused on stable relationships and routes. Put detailed commands, runtime sequences, wire
  formats, and algorithm rationale in the document that owns that concern.
- Preserve an established public path when splitting a subject so existing routes remain stable.
- Update the nearest hub and the root README documentation list when adding, moving, or removing a human-facing
  document.
- Keep the graph at catalog-to-document or catalog-to-hub-to-detail depth. Do not build recursive indexes.

## Placing Or Growing Documentation

1. Name the durable fact and the concrete reader task.
2. Decide whether the audience is a human contributor or an AI agent.
3. For a project fact, use the root README documentation list to identify its human-facing owner. For agent policy, use
   the applicable root or build-root agent guide.
4. Extend an existing document only when the new material answers the same reader question and has the same lifecycle.
5. Add a bounded detail page when the concern is independently selectable and would otherwise make readers load
   unrelated material.
6. Create a stable subject hub only when at least two bounded concerns share a durable scope.
7. Around 200 lines or 16 KB, review cohesion rather than splitting automatically.
8. Treat a page as a catchall when it cannot state one scope boundary or mixes concerns, evidence types, lifecycles, or
   audiences that readers would not normally need together.
9. Update the nearest route and verify changed links after reorganizing.

## Human, Agent, And Private Documentation

- Keep human-facing project documentation stable, contributor-facing, and safe to publish.
- Keep tracked agent policy under `AGENTS.md` and `agent-guides/`, outside the human documentation routes.
- Keep plans, design exploration, audit handoffs, command logs, review packages, and work-in-progress status under
  ignored `.codex/audits/<audit-name>/` paths.
- Do not make private coordination state a prerequisite for understanding, building, testing, or contributing to the
  tracked project.
- Distill accepted project facts into the human architecture, runtime, development, or algorithm document that owns
  them. Distill durable AI instructions into the applicable agent guide.
- Human-facing docs describe current behavior and intended architecture. They are not a timeline of how a refactor or
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
