# Parameter Flow Handoff

This is the current restart point for a fresh bounded implementation chat.

## Current Understanding

The audit found that parameter declarations and mappings are highly mechanical. Dynamic config already has a repeated
pattern of config fields, accessor trait, concrete accessor impl, `Parameter` constants, visitor methods, and traversal.
The user explicitly called out that this is a good fit for a macro and wants an implementer-ready artifact for the macro
writing step.

The first dynamic-config-only `macro_rules!` proof was rejected and scratched from production code. The chosen direction
is now a generic attribute proc-macro API that keeps config structs as ordinary Rust and generates mechanical companion
items.

## Durable Context To Read First

- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/audit.md`
- `docs/parameter-flow-audit.md`
- `rust/AGENTS.md`
- `docs/development.md`

## Rejected Slice: Dynamic Parameter Group Macro Proof

The previous slice brief below is retained as historical context only. Do not execute it again.

The attempted implementation used a dynamic-specific `macro_rules!` macro and a custom invocation DSL. That shape was
rejected because it hid a large body of Rust in a group-specific macro and made the source-of-truth list noisier than the
original code.

Production code was restored to the pre-proof version.

## Slice Brief: Attribute Parameter Group Macro Prototype

### Objective

Implement a narrow production prototype of the chosen generic attribute proc-macro API and apply it only to
`DynamicBPMDetectionConfig`.

The implementation must avoid the rejected dynamic-specific `macro_rules!` shape. The source of truth should be the
ordinary Rust config struct plus small field attributes. From that struct, generate the current dynamic-config companion
items:

- accessor trait getter/setter methods;
- accessor impl for `()`;
- accessor impl for the concrete config;
- `Parameters<Config>` companion type and `Parameter` constants;
- defaults and validation;
- visitor/traversal matching the current public API.

Keep the current public type names, parameter labels, ranges, defaults, units, steps, logarithmic flags, visitor order,
serde schema, and validation behavior.

### Non-goals

- Do not migrate static BPM, normal distribution, or GUI config.
- Do not change runtime sync behavior in desktop, wasm, or plugin code.
- Do not change config file schema, serde field names, public type names, parameter labels, ranges, defaults, units, steps,
  or logarithmic flags.
- Do not propose another group-specific declarative macro invocation that replaces the Rust struct.
- Do not require a custom mini-language where ordinary struct fields would do.
- Do not replace the visitor with a new catalog/enumeration in this slice.
- Do not solve static BPM computed methods in this slice.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially "Rejected Macro Proof", "Revised Macro Requirements", and "Chosen
  Macro API Direction".
- `docs/parameter-flow-audit.md`, especially "Typed Parameter Inventory" and "Invariant And Failure-Mode Audit".
- `rust/AGENTS.md`, especially the macro and Rust workspace instructions.

### Likely files / areas

- `rust/Cargo.toml`
- `rust/crates/parameter_macros/Cargo.toml` or similarly named new proc-macro crate
- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/bpm_detection_core/Cargo.toml`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- Existing dynamic parameter tests in `rust/crates/bpm_detection_core/src/parameters.rs`
- `docs/audits/parameter-flow/handoff.md` for the required back-handoff

### Relevant boundaries / integration points

- The new macro crate must not make `bpm_detection_core` depend on egui, nih-plug, desktop, wasm, or plugin crates.
- The runtime `parameter` crate remains the owner of `Parameter`, `Asf64`, and `OnOff`.
- Generated APIs must preserve the core API consumed by `gui`, `desktop`, `wasm`, and `midi-bpm-detector-plugin` unless a
  later slice explicitly chooses a breaking change.
- Plugin code depends on exact dynamic parameter constants, visitor names, labels, IDs supplied manually at plugin
  construction, and `read_dynamic_config()` traversal behavior.
- Serde field names for config structs must remain ordinary struct field names.

### Expected behavioral change

None intended.

This is a structural refactor of dynamic parameter declarations only. The macro-generated code should be behaviorally
equivalent to the current hand-written dynamic-config machinery.

### Expected structural change

- Add a small proc-macro crate for `#[parameter_group(...)]` and field-level `#[parameter(...)]` metadata.
- Annotate `DynamicBPMDetectionConfig` as the source of truth for dynamic parameter fields.
- Remove the now-generated hand-written dynamic accessor trait, accessor impls, parameters type, constants, visitor trait,
  `Default`, and `validate` code from `parameters.rs`, while preserving their public generated equivalents.
- Keep static BPM, normal distribution, and GUI config hand-written.

### Acceptance criteria

- The rejected macro attempt is absent from `rust/crates/bpm_detection_core/src/parameters.rs`.
- The dynamic config source remains an ordinary Rust struct with real fields.
- There is no `dynamic_bpm_detection_parameter_group!` or equivalent group-specific DSL.
- `DynamicBPMDetectionConfigAccessor`, `DefaultDynamicBPMDetectionParameters`,
  `DynamicBPMDetectionParameters<Config>`, and `DynamicBPMDetectionParameterVisitor<Config>` remain available under the
  same names.
- Dynamic parameter constants keep the same names, labels, units, ranges, steps, logarithmic flags, and defaults.
- `DynamicBPMDetectionParameters::<Config>::visit` keeps the current traversal order:
  `beats_lookback`, `normal_distribution_weight`, `time_distance_weight`, `velocity_current_note_weight`,
  `velocity_note_from_weight`, `in_beat_range_weight`, `multiplier_weight`, `subdivision_weight`,
  `octave_distance_weight`, `pitch_distance_weight`, `high_tempo_bias_weight`.
- Existing dynamic parameter inventory tests still pass.
- Downstream crates still compile against the generated dynamic API.

### Tests / checks

- From `rust/`: `cargo test -p bpm_detection_core parameter_inventory_tests`
- From `rust/`: `cargo test -p bpm_detection_core`
- From `rust/`: run the narrowest extra compile/test command needed if downstream crates fail to compile after the API
  generation change.

### Risks / open questions

- Proc-macro diagnostics must point at useful field attributes; confusing generated errors are a reason to stop and
  report back.
- The macro may need an explicit `parameter_crate = parameter` path override if generated paths are brittle.
- Field metadata can still become noisy. Keep required metadata minimal: `label`, `range`, and `default`; use defaults for
  `unit`, `step`, `logarithmic`, setter names, and const names where possible.
- `OnOff<T>` defaults and `Duration`-style values must remain valid expressions in generated `Parameter::new` constants.
- Static BPM config has computed accessor methods and should not be forced into this prototype.

### Back-handoff requirements

Update `docs/audits/parameter-flow/handoff.md` with a back-handoff section containing:

- status: complete / partial / blocked;
- branch and commit if applicable;
- files changed;
- summary of the implemented macro API shape;
- examples of the annotated dynamic config source shape;
- tests/checks run and results;
- any deviations from this brief;
- macro implementation decisions and rejected alternatives;
- remaining risks or recommended next slice.

Also update `docs/audits/parameter-flow/audit.md` if the implementation makes a durable design decision that future
slices must know.

## Prompt For Fresh Implementer Chat

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/repo-map.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/handoff.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Attribute Parameter Group Macro Prototype" from docs/audits/parameter-flow/handoff.md.

Implement a generic attribute proc-macro prototype and apply it only to DynamicBPMDetectionConfig. Do not migrate static
BPM, normal distribution, or GUI config. Do not change runtime sync behavior, serde schemas, public names, labels,
ranges, defaults, units, steps, logarithmic flags, or visitor order. The previous dynamic-specific macro_rules! proof was
rejected; do not repeat that shape. Keep the config struct as ordinary Rust with small field metadata attributes, and
update docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Dynamic Parameter Group Macro Proof

### Status

Rejected and scratched.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none.

### Files changed

- Attempted: `rust/crates/bpm_detection_core/src/parameters.rs`
- Cleanup: `rust/crates/bpm_detection_core/src/parameters.rs` restored to `upstream/main`
- Docs updated: `docs/audits/parameter-flow/audit.md`, `docs/audits/parameter-flow/handoff.md`

### Summary

The attempted implementation replaced the hand-written dynamic BPM parameter boilerplate with a local,
dynamic-specific `dynamic_bpm_detection_parameter_group!` `macro_rules!` macro and a large invocation DSL.

The user rejected that shape. The production file has been restored to the pre-proof version.

### Behavioral changes

None retained. The attempted production change was removed.

### Structural changes

None retained in production Rust.

### Affected boundaries / integration points

- Production dynamic parameter API is back to the original hand-written version.

### Tests / checks

Cleanup check:

- `git diff -- rust/crates/bpm_detection_core/src/parameters.rs` is empty after restoring the file.

No cargo tests were run after cleanup because production Rust changes were removed.

### Decisions made

- A domain-specific macro that generates the whole config/trait/parameter/visitor block is not acceptable.
- A large invocation DSL containing field, const, setter, and metadata is not acceptable.
- The desired shape is a generic derive/attribute-style facility over ordinary Rust structs.
- Superseded by the later audit decision to use an attribute proc macro for the first dynamic-config prototype.

### Deviations from brief

The slice brief was too permissive about local `macro_rules!` and did not forbid a custom mini-language. Future briefs
must require ordinary Rust structs and generic macro ergonomics.

### Remaining risks

- Historical note: at the time of this rejected proof, the macro shape was still undecided.
- The later audit decision chose an attribute proc macro and preserved generated visitors for the first prototype.

### Recommended next slice

Superseded. The active slice is now "Attribute Parameter Group Macro Prototype".

## Back-Handoff: Attribute Parameter Group Macro Prototype

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none.

### Files changed

- `rust/Cargo.toml`
- `rust/Cargo.lock`
- `rust/crates/parameter_macros/Cargo.toml`
- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/parameter_group.rs`
- `rust/crates/bpm_detection_core/Cargo.toml`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Added a generic `#[parameter_group(...)]` attribute proc-macro prototype in the new `parameter_macros` crate and applied
it only to `DynamicBPMDetectionConfig`.

The annotated dynamic config remains an ordinary Rust struct. Each dynamic field now carries small `#[parameter(...)]`
metadata for its existing label, range, default, and only non-default flags such as `step = 1.0` or
`logarithmic = true`.

Example source shape:

```rust
#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
pub struct DynamicBPMDetectionConfig {
    #[parameter(label = "Beats Lookback", range = 2.0..=32.0, step = 1.0, default = 8)]
    pub beats_lookback: u8,
}
```

The macro generates the current dynamic public companion API:

- `DynamicBPMDetectionConfigAccessor`
- `impl DynamicBPMDetectionConfigAccessor for ()`
- `impl DynamicBPMDetectionConfigAccessor for DynamicBPMDetectionConfig`
- `DefaultDynamicBPMDetectionParameters`
- `DynamicBPMDetectionParameters<Config>`
- dynamic `Parameter` associated constants
- `DynamicBPMDetectionParameterVisitor<Config>`
- `DynamicBPMDetectionParameters::<Config>::visit`
- `DynamicBPMDetectionConfig::default`
- `DynamicBPMDetectionConfig::validate`

### Behavioral changes

None intended.

Runtime sync behavior, serde field names/schema, public dynamic type names, labels, ranges, defaults, units, steps,
logarithmic flags, and visitor order are preserved.

### Structural changes

- Added `rust/crates/parameter_macros` as a proc-macro crate using `syn`, `quote`, and `proc-macro2`.
- Added `parameter_macros` as a dependency of `bpm_detection_core`.
- Removed the hand-written dynamic accessor trait, accessor impls, parameter companion type/constants, visitor trait,
  default impl, and validation impl from `parameters.rs`; those items are now generated from the annotated dynamic config
  struct.
- Kept static BPM, normal distribution, and GUI config hand-written.

### Affected boundaries / integration points

- `bpm_detection_core` now depends on the local `parameter_macros` proc-macro crate.
- The macro crate depends on the runtime `parameter` crate only for tests; generated code references `parameter` by path.
- GUI and plugin crates continue consuming the same generated dynamic public API.
- No egui, nih-plug, desktop, wasm, or plugin dependencies were added to `parameter_macros` or the runtime `parameter`
  crate.

### Tests / checks

- Red check before implementation: `cargo test -p parameter_macros` failed because `parameter_group` and generated
  companion names were missing.
- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 2 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.

### Decisions made

- Implemented this as an attribute proc macro, not a dynamic-specific `macro_rules!` macro or field-list DSL.
- Required explicit group-level public item names instead of deriving names implicitly.
- Field metadata requires `label`, `range`, and `default`; `unit`, `step`, `logarithmic`, `const_name`, and `setter`
  are optional.
- Generated validation directly calls each generated `Parameter` constant in field order. This preserves first-error
  behavior without coupling the generic macro to the old private `ConfigParameterValidator` helper.
- Left visitor default methods unchanged for compatibility; tightening visitor exhaustiveness remains a later slice.

### Deviations from brief

None intentional.

### Remaining risks

- Proc-macro diagnostics are intentionally basic in this prototype. A coordinator review should decide whether to invest
  in richer spans or clearer duplicate/missing metadata messages before wider migration.
- Generated getters return field values by copy, matching the current dynamic fields. A wider migration should revisit
  whether future parameter value types need clone-based accessors or a different accessor shape.
- The macro assumes generated code can reference the runtime crate as `parameter` unless `parameter_crate = ...` is
  provided.

### Recommended next slice

Coordinator review of the generated API shape, diagnostics, rust-analyzer ergonomics, and diff readability before
migrating `NormalDistributionConfig` or `GUIConfig`.
