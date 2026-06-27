# Parameter Audit Handoff

This file is the restart point for future audit-coordinator or implementation chats working on the parameter mapping/refactor
thread.

## Current Status

Branch: `codex/parameter-flow-audit`

Current detailed audit doc: `docs/parameter-flow-audit.md`

Canonical coordinator workspace:

- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/handoff.md`

Completed audit steps:

1. Parameter declaration inventory.
2. Flow trace from config/defaults through egui, plugin host params, remote controls, runtime tasks, worker snapshots, and
   atomics.
3. Invariant and failure-mode audit.

Current branch checkpoint:

- `docs/parameter-flow-audit.md`
- `docs/parameter-audit-handoff.md`
- `docs/audits/parameter-flow/`
- `rust/crates/parameter_macros/`
- dynamic-config macro wiring in `rust/crates/bpm_detection_core/src/parameters.rs`

The dynamic-config-only attribute proc-macro prototype is implemented. Static BPM, normal distribution, and GUI config
remain hand-written.

## Key Findings So Far

- There are 20 typed `Parameter` declarations:
  - 2 GUI/display;
  - 3 static BPM model;
  - 4 normal distribution static submodel;
  - 11 dynamic scoring.
- Dynamic scoring has the strongest shared traversal, but the visitor methods have defaults, so missing visitor behavior
  can still compile.
- Static BPM, normal distribution, GUI/display, and output/runtime state are much more manually wired.
- GUI/display interpolation params currently ride dynamic update paths in desktop, wasm, and plugin modes.
- Plugin host-origin dynamic sync is overloaded: dynamic scoring, GUI/display, and `send_tempo` all share the dynamic
  task path.
- Plugin host-origin dynamic sync assigns `interpolation_duration` twice.
- Parameter-like atomics such as `send_tempo`, `enable_midi_clock`, and `daw_port` have bespoke persistence/sync rules.

## Current Discussion Point

The next audit step was originally:

> Evaluate the current trait/visitor approach against the traced flows.

The user then raised a concrete concern: the accessor traits, concrete config impls, parameter constants, and visitor
traversals are highly mechanical. Dynamic config has this pattern, but static config, normal distribution config, and GUI
config do not have the same homogeneous machinery because the boilerplate becomes awkward.

Current leaning:

- A macro-generated parameter group may still be the natural next design step.
- The first dynamic-specific `macro_rules!` proof was rejected and removed from production code.
- The chosen direction is a generic attribute proc-macro API that keeps config structs as ordinary Rust and generates
  mechanical companion items.
- The first implementation prototype should apply only to `DynamicBPMDetectionConfig` and preserve all existing public
  names and behavior.
- Do not let the macro decide runtime sync policy. Static/dynamic/gui/output update semantics should remain explicit.

## Recommended Next Audit-Coordinator Step

Use `$repo-audit-coordinator`.

Continue with:

1. Use the canonical workspace under `docs/audits/parameter-flow/`.
2. Review the `Attribute Parameter Group Macro Prototype` back-handoff in `docs/audits/parameter-flow/handoff.md`.
3. Decide whether to improve macro diagnostics/DX before applying the pattern to normal distribution or GUI config.

## Prompt To Start The Next Audit-Coordinator Chat

```text
[$repo-audit-coordinator] Use the repo audit coordinator flow.

Read:
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/handoff.md`
- `docs/parameter-audit-handoff.md`
- `docs/parameter-flow-audit.md`

We are continuing the parameter mapping/refactor audit. Review the back-handoff for the attribute parameter group macro
prototype, update the audit docs, and recommend whether to improve macro diagnostics/DX before applying the pattern next
to normal distribution and GUI config.
```

## Prompt To Start A Future Implementer Chat

```text
[$bounded-implementer] Use the bounded implementer flow.

Read:
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/handoff.md`
- `docs/parameter-audit-handoff.md`
- `docs/parameter-flow-audit.md`

Execute the next bounded slice selected by the audit coordinator. The dynamic-config-only attribute proc-macro prototype
already exists; do not repeat the rejected dynamic-specific `macro_rules!` proof, and do not migrate static BPM, normal
distribution, or GUI config unless that slice explicitly asks for it.
```
