# AGENTS.md

Repository-level instructions for AI coding agents working on this monorepo.

## Read These First

- Follow `docs/engineering-style.md` for general implementation style.
- Follow `docs/refactoring-guide.md` for refactors, lint-driven work, and structural cleanup.
- Follow `agent-guides/repo-structure.md` for monorepo shape and cross-build-root architecture.
- Follow `agent-guides/documentation.md` for documentation routing and wording rules.
- Follow `agent-guides/audit-workflow.md` for long-running audit/refactor workflow.
- Follow build-root instructions before editing a component:
  - `rust/AGENTS.md` for Rust workspace work.
  - `extension/AGENTS.md` for Kotlin/Bitwig extension work.

## Hard Rules

- Make small, reviewable changes. Prefer commits that map to one clear design or behavior step.
- Discuss non-obvious design choices before implementing them.
- Do not revert unrelated changes. Assume unrecognized local changes came from the user.
- When changing behavior, verify with the narrowest useful command first, then broaden verification when the blast radius grows.
- Keep Rust and Kotlin as separate build roots. Do not create a root mega-build that makes Cargo own the Kotlin extension
  or Gradle own the Rust workspace.
- Keep the Bitwig tempo bridge narrow. Do not turn it into a general remote-control protocol unless a concrete feature
  needs that.
- Do not rely on chat memory as the source of truth. Before assuming a change is local, check relevant component
  boundaries, shared contracts, generated code, build/test/CI paths, runtime configuration, and integration points.

## Instruction And Command Consistency

- If an instruction, documented command, or workflow note does not work, first check whether the instruction is stale,
  incomplete, or being run from the wrong build root. Do not add compatibility code, wrappers, build plumbing, or product
  behavior just to make a stale instruction true.
- Prefer fixing the instruction or documentation when the code is already in the right shape. If changing code or tooling
  still seems like the right fix, make the reason explicit and ask the human before doing it unless the task already
  requested that exact tooling change.
- Keep agent-facing instructions current and concise. When adding a new rule, remove or update any nearby stale wording
  that would point agents in a different direction.
- When asking the human for direction, ask a concrete action question with the options or consequence named. Do not hand
  back vague uncertainty.
