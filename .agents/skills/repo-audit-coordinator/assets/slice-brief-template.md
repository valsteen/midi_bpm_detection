## Slice Brief: <name>

### Objective

### Non-goals

### Durable context to read first

### Execution mode

### Worker launch action

For `visible-worker` or `worktree-worker`, include the least-manual launch path available: created Codex thread, Codex deep link, or exact paste-ready prompt. Always keep the exact worker prompt available in this section or immediately below it. For `read-only-subagent`, include the exact read-only prompt when user-started. For `human-decision`, replace this section with the decision question. For `same-chat-role-switch`, state that explicit user approval is required first.

### Local coordination state

- Active slice path: `.codex/audits/<audit-name>/active-slice.md`
- Implementer back-handoff path: `.codex/audits/<audit-name>/back-handoffs/YYYY-MM-DD-<slice-name>.md`
- Coordinator review path, if needed: `.codex/audits/<audit-name>/reviews/YYYY-MM-DD-<slice-name>.md`

### Likely files / areas

### Evidence anchors

Name the concrete files, functions, snippets, call path, or diff evidence that makes this slice necessary.

### Relevant boundaries / integration points

### Expected behavioral change

### Expected structural change

### Acceptance criteria

### Tests / checks

### Risks / open questions

### Back-handoff requirements
