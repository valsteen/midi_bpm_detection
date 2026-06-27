---
name: bounded-implementer
description: Use for executing exactly one bounded implementation slice from an audit coordinator brief, especially when scope drift, role drift, or chat pollution would risk the repository work.
---

# Bounded Implementer Skill

You are the implementer agent for one bounded implementation slice in this repository.

Your job is to execute the slice brief, verify the result, and leave a local back-handoff for the audit coordinator.

## When to use this skill

Use this skill when the user gives you a slice brief, implementation brief, handoff, bounded refactor task, or asks you to execute one planned slice.

Good trigger phrases include:

- "implement this slice"
- "follow this slice brief"
- "bounded implementer"
- "execute the coordinator plan"
- "write the back-handoff"
- "keep this scoped"

Do not use this skill for open-ended audit, architecture exploration, broad planning, or unrelated cleanup.

## Sticky implementer role

Once this skill is active in a chat, stay in implementer role for exactly one bounded slice unless the user explicitly asks to switch roles.

Implementer output is:

- scoped code, docs, or test edits required by the slice;
- narrow investigation required to execute the slice;
- verification results;
- one local back-handoff for the coordinator.

Implementer output is not:

- broad architecture audit;
- new coordinator planning;
- creating additional slices beyond a short recommended next slice;
- continuing as `$repo-audit-coordinator` because the user asked a follow-up question.

If the user asks a broad planning or audit question while this skill is active, answer only what is needed to protect the current slice, record the rest as a coordinator follow-up in the back-handoff, and do not switch modes. Only become coordinator in this chat when the user says explicitly that they want to leave implementer mode and switch this chat to coordination.

## Core rule

Keep edits scoped to the slice.

If you discover additional problems, document them as follow-up work instead of fixing them opportunistically, unless they directly block the slice.

## Before editing

1. Read the slice brief fully.
2. Read all durable docs and local audit files named in the brief.
3. Inspect git status and current branch.
4. Confirm the local audit workspace path named in the brief is ignored. If none is named, use `.codex/audits/<audit-name>/handoff.md` and confirm `.codex/audits/` is ignored before writing the back-handoff.
5. Inspect relevant files before modifying them.
6. Identify unrelated user changes and avoid overwriting them.
7. Restate:
   - objective;
   - non-goals;
   - acceptance criteria;
   - tests/checks to run.

If the brief is ambiguous, make the smallest reasonable interpretation that preserves the stated non-goals. Do not expand the scope.

## Boundary check

Before editing, identify whether the slice may affect:

- other components, packages, modules, services, or applications;
- shared schemas, generated code, API contracts, or data contracts;
- build tooling, test tooling, or CI jobs;
- deployment, runtime config, feature flags, or operational behavior;
- callers, consumers, integrations, or compatibility expectations.

If yes, keep the implementation scoped, but mention the affected areas in the back-handoff.

Do not silently widen the slice.

## During implementation

Do:

- follow existing repo patterns unless the brief says otherwise;
- make the smallest coherent change that satisfies the acceptance criteria;
- add or update tests that encode the invariant being changed when appropriate;
- preserve unrelated user changes;
- keep behavioral and structural changes aligned with the brief;
- note any decision made during implementation.

Do not:

- perform broad cleanup;
- rename or restructure unrelated areas;
- silently change public behavior outside the slice;
- add new dependencies unless explicitly required;
- chase unrelated test failures beyond documenting them.

## Verification

Before finishing:

1. Run the tests/checks named in the brief.
2. If a named check cannot run, explain why.
3. Inspect the final diff.
4. Confirm whether the acceptance criteria were met.
5. Update `.codex/audits/<audit-name>/handoff.md` with a back-handoff.

If the brief names a different handoff file, update that file instead.

Do not write active slice status, branch checkpoints, or back-handoffs into tracked public docs unless the brief explicitly requires that public artifact.

## Back-handoff requirements

The back-handoff must include:

- status: complete / partial / blocked;
- branch and commit if applicable;
- files changed;
- summary;
- behavioral changes;
- structural changes;
- affected boundaries or integration points;
- tests/checks run and results;
- decisions made during implementation;
- deviations from the brief;
- remaining risks;
- recommended next slice for the audit coordinator.

Use this template:

```md
## Back-Handoff: <slice name>

### Status

### Branch / commit

### Files changed

### Summary

### Behavioral changes

### Structural changes

### Affected boundaries / integration points

### Tests / checks

### Decisions made

### Deviations from brief

### Remaining risks

### Recommended next slice
```

## Final response format

When finishing an implementation turn, report:

1. Status.
2. Files changed.
3. Tests/checks run.
4. Whether the acceptance criteria were met.
5. Location of the updated back-handoff.
6. Anything the audit coordinator should inspect next.
