# Audit And Refactor Workflow

Detailed long-running audit and refactor instructions for agents. Start from `../AGENTS.md`; this file holds the longer
workflow rules so the entrypoint stays small.

For long-running architecture, migration, audit, or refactor work, this repository can use repo-scoped Codex skills.

- Use `$repo-audit-coordinator` for planning, slicing, decision logging, and maintaining audit continuity.
- Use `$bounded-implementer` for executing exactly one bounded implementation slice from a coordinator brief.

Keep active audit coordination state under the ignored local workspace `.codex/audits/<audit-name>/`.
This includes active slice briefs, fresh-context handovers, implementation back-handoffs, branch checkpoints, command
logs, and work-in-progress status.

Only promote long-lived project documentation or enduring AI instructions into tracked public docs. When an older branch
already has tracked transient audit artifacts, migrate them into `.codex/audits/<audit-name>/` before appending new
coordination state.

Do not rely on chat memory as the source of truth. Before assuming a change is local, check relevant component
boundaries, shared contracts, generated code, build/test/CI paths, runtime configuration, and integration points.
