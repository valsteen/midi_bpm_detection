# Parameter Flow Handoff

This is the current restart point for a fresh bounded implementation chat.

## Current Understanding

The audit found that parameter declarations and mappings are highly mechanical. Dynamic config already has a repeated
pattern of config fields, accessor trait, concrete accessor impl, `Parameter` constants, visitor methods, and traversal.
The user explicitly called out that this is a good fit for a macro.

The first dynamic-config-only `macro_rules!` proof was rejected and scratched from production code. The chosen direction
is a generic attribute proc-macro API that keeps config structs as ordinary Rust and generates mechanical companion
items. The dynamic-config-only prototype and first diagnostics follow-up are now committed.

The dynamic metadata-spec split, `NormalDistributionConfig` macro migration, `GUIConfig` macro migration, static BPM
computed-method split, `StaticBPMDetectionConfig` macro migration, GUI settings visitor adoption, visitor consumer
homogeneity audit, and normal-distribution order alignment are now implemented. All typed parameter groups now use the
generic attribute macro. Normal-distribution generated traversal, egui settings, plugin parameter construction,
host-origin copy-back, `LiveConfig` accessors, and CLAP remote controls now use the canonical GUI order:
`std_dev`, `resolution`, `cutoff`, `factor`. The next recommended slice is a plugin host mapping surface audit/helper
decision.

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

## Historical Prompt For Fresh Implementer Chat: Attribute Parameter Group Macro Prototype

This prompt is retained as historical context for the completed prototype slice. Do not use it as the next implementer
prompt.

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

Superseded. The "Attribute Parameter Group Macro Prototype" slice is now complete.

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

## Back-Handoff: Parameter Group Macro Diagnostics DX

### Status

Complete for the first diagnostics slice.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Base commit for this follow-up: `431d0d3 Add dynamic parameter group macro prototype`.

Commit: none.

### Files changed

- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/diagnostics.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Improved the `#[parameter_group]` parser diagnostics so group-level and field-level attribute errors use distinct
terminology and better spans. Added local compile-fail-style tests that build small fixture crates offline and assert the
important stderr fragments.

### Behavioral changes

No runtime behavior changes intended. Existing generated parameter API behavior is unchanged.

Macro misuse diagnostics changed:

- missing field-level `default` now reports ``missing required argument `default` in #[parameter(...)]``;
- missing field-level required arguments now underline the full `#[parameter(...)]` attribute instead of only the
  attribute path token;
- unknown field-level keys now report ``unknown argument `<key>` in #[parameter(...)]``;
- `ranges` now suggests `range`;
- duplicate field-level keys now mention `#[parameter(...)]`;
- unknown group-level keys now mention `#[parameter_group(...)]`.

### Structural changes

- Added `tests/diagnostics.rs` as a lightweight trybuild-style harness without adding a new external dependency.
- The harness writes temporary fixture crates under `rust/target/parameter-macro-diagnostic-fixtures/`, runs
  `cargo check --offline`, and checks stderr fragments.
- `ParameterArgs` now stores the field-level `#[parameter(...)]` attribute so missing required arguments can use the full
  attribute span.
- Added macro crate documentation describing the generated public contract and the current `Parameters<()>` compatibility
  bridge.

### Affected boundaries / integration points

- The public macro API is unchanged.
- The generated public dynamic parameter API is unchanged.
- Tests spawn nested Cargo checks, so future maintainers should keep fixture manifests isolated with an empty
  `[workspace]` table and offline mode.

### Tests / checks

- Red run: `cargo test -p parameter_macros --test diagnostics` initially failed after reaching the macro diagnostics:
  missing `default` pointed at `#[parameter_group(...)]`, unknown keys used generic wording, and duplicate field keys used
  group wording.
- Green run: `cargo test -p parameter_macros --test diagnostics` passed, 4 tests.
- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.
- Follow-up red run for underline width:
  `cargo test -p parameter_macros --test diagnostics missing_default_reports_field_attribute_span` failed while the
  missing-`default` underline still covered only one character.
- Follow-up green run for underline width:
  `cargo test -p parameter_macros --test diagnostics missing_default_reports_field_attribute_span` passed after switching
  missing required field arguments to `syn::Error::new_spanned` over the full `Attribute`.

### Decisions made

- Did not add `trybuild` as a dependency in this slice; the custom harness avoids network/dependency churn while covering
  the important stderr regressions.
- Kept the `impl Accessor for ()` compatibility bridge for now. It is public but intended only for metadata constants;
  replacing it cleanly likely needs a `ParameterSpec<T>` or similar split in the runtime `parameter` crate.
- Did not implement bare boolean flags, integer-range syntax normalization, derive macro conversion, or convention-based
  naming in this slice.

### Deviations from brief

This implements the high-priority diagnostic subset rather than every suggested compile-fail case. Semantic checks such
as `range_start_greater_than_end`, `default_outside_range`, and `logarithmic_with_zero_or_negative_range` remain open
because the current macro accepts arbitrary Rust expressions for those values.

### Remaining risks

- The diagnostic harness asserts stderr fragments rather than full snapshots.
- Field-level semantic validation is still mostly deferred to generated Rust types and runtime config validation.
- The `Parameters<()>` bridge can still panic if downstream code explicitly calls `Parameter::get` or `Parameter::set`
  on the default metadata catalog.

### Recommended next slice

Either design the `ParameterSpec<T>` metadata-only split for default catalogs, or continue smaller macro-DX improvements
with bare boolean flags and integer literal range syntax.

## Coordinator Review: Attribute Macro Prototype And Diagnostics

### Status

Verified.

### Branch / commits

Branch: `codex/parameter-flow-audit`, two commits ahead of `upstream/main`.

Commits:

- `431d0d3 Add dynamic parameter group macro prototype`
- `a5fc659 Improve parameter macro diagnostics`

### Review result

The committed implementation matches the accepted direction. `DynamicBPMDetectionConfig` remains an ordinary Rust struct
with small field-level `#[parameter(...)]` metadata. The generated dynamic public API preserves the expected names and
visitor order, and downstream GUI/plugin tests compile against it.

The diagnostics follow-up is also in-scope: it addresses the prototype's highest-risk ergonomics before wider migration
and adds local fixture-based tests without adding an external compile-fail test dependency.

### Fresh verification

Run from `rust/` unless noted:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, including 4 diagnostics tests and 2 parameter-group tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- From repo root, `git diff --check`: passed.

### Remaining risk to address before wider migration

The generated default catalog is still the public alias `DefaultDynamicBPMDetectionParameters =
DynamicBPMDetectionParameters<()>`. That means metadata-only constants still carry fake `get`/`set` function pointers
through an `impl DynamicBPMDetectionConfigAccessor for ()` that panics if used as a real config.

Today this is mostly contained: call sites use default dynamic parameters for labels, defaults, ranges, visitor ordering,
and plugin host/remote-control ordering. Still, the type advertises a real `Parameter<(), T>`, which is misleading and
easy to misuse. Split that before migrating more groups.

## Slice Brief: Metadata-Only Dynamic Parameter Specs

### Objective

Introduce a metadata-only parameter spec shape for generated dynamic default catalogs so
`DefaultDynamicBPMDetectionParameters` no longer exposes `Parameter<(), T>` values or requires a generated
`DynamicBPMDetectionConfigAccessor for ()` bridge.

Keep `DynamicBPMDetectionParameters<Config>` as the config-bound `Parameter<Config, T>` catalog used by real config
read/write paths.

### Non-goals

- Do not migrate static BPM, normal distribution, or GUI config to the macro in this slice.
- Do not remove the hand-written `DefaultStaticBPMDetectionParameters`, `DefaultNormalDistributionParameters`, or
  `DefaultGUIParameters` fake-config aliases yet.
- Do not change runtime sync behavior in desktop, wasm, or plugin code.
- Do not change dynamic serde schemas, public config fields, labels, ranges, defaults, units, steps, logarithmic flags, or
  visitor order.
- Do not replace the visitor pattern with a new catalog/enumeration.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially "Coordinator Review: Attribute Proc-Macro Prototype".
- `docs/audits/parameter-flow/handoff.md`, especially the prototype and diagnostics back-handoffs.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/parameter/src/lib.rs`
- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/parameter_group.rs`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameter_adapters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Relevant boundaries / integration points

- The runtime `parameter` crate should own the new metadata-only shape, tentatively `ParameterSpec<ValueType>`.
- The macro crate should generate dynamic metadata specs and config-bound `Parameter<Config, ValueType>` constants from
  the same field metadata.
- `bpm_detection_core` must not gain egui, nih-plug, desktop, wasm, or plugin dependencies.
- Plugin dynamic remote controls and host reads currently use `DefaultDynamicBPMDetectionParameters::visit` only for
  ordering. They can switch to a real config-bound dynamic catalog, or to an explicitly metadata/spec visitor if the
  slice adds one.
- Static, normal, and GUI fake-config aliases remain known debt for later slices.

### Expected behavioral change

None intended.

This is a structural cleanup of generated dynamic metadata only. Runtime config reads/writes, plugin host parameter
values, GUI rendering, validation, and dynamic visitor order should stay equivalent.

### Expected structural change

- Add a metadata-only parameter spec type in `parameter`.
- Update `parameter_macros` so the generated default dynamic catalog uses specs rather than `Parameter<(), T>`.
- Remove generated `impl DynamicBPMDetectionConfigAccessor for ()` from the macro output.
- Keep generated `DynamicBPMDetectionParameters<Config>` constants as config-bound `Parameter<Config, T>` values.
- Update dynamic plugin/test call sites that used the `()` catalog for ordering or metadata.

### Acceptance criteria

- `DefaultDynamicBPMDetectionParameters::*` exposes metadata-only specs, not `Parameter<(), T>`.
- The generated dynamic macro output no longer includes `impl DynamicBPMDetectionConfigAccessor for ()`.
- `DynamicBPMDetectionParameters<DynamicBPMDetectionConfig>` and generic
  `DynamicBPMDetectionParameters<Config>` remain available for config-bound parameters.
- Existing dynamic labels, ranges, defaults, units, steps, logarithmic flags, and visitor order are preserved.
- Plugin dynamic remote controls still expose every dynamic parameter in the same order.
- Plugin dynamic host reads still round-trip host parameter values into `DynamicBPMDetectionConfig`.
- Static BPM, normal distribution, and GUI default aliases are not migrated in this slice.

### Tests / checks

- From `rust/`: `cargo test -p parameter`
- From `rust/`: `cargo test -p parameter_macros`
- From `rust/`: `cargo test -p bpm_detection_core parameter_inventory_tests`
- From `rust/`: `cargo test -p bpm_detection_core`
- From `rust/`: `cargo test -p gui`
- From `rust/`: `cargo test -p midi-bpm-detector-plugin`
- From `rust/`:
  `cargo clippy -p parameter -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`
- From repo root: `git diff --check`

### Risks / open questions

- Naming may settle on `ParameterSpec`, `ParameterMetadata`, or a similar term. Prefer a name that makes "no config
  accessors here" obvious.
- If a metadata visitor is introduced, keep it small and do not replace the existing config-bound visitor in this slice.
- Generated code should avoid duplicating metadata literals between specs and config-bound parameters more than necessary.
- The hand-written static/normal/gui `Parameters<()>` bridges remain after this slice; document that as deliberate
  follow-up debt, not an accidental omission.

### Back-handoff requirements

Update `docs/audits/parameter-flow/handoff.md` with:

- status: complete / partial / blocked;
- branch and commit if applicable;
- files changed;
- final metadata type/API shape;
- how dynamic plugin ordering/read call sites changed;
- tests/checks run and results;
- deviations from this brief;
- remaining risks;
- recommended next slice.

## Prompt For Fresh Implementer Chat: Metadata-Only Dynamic Parameter Specs

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Metadata-Only Dynamic Parameter Specs" from docs/audits/parameter-flow/handoff.md.

Introduce a metadata-only parameter spec shape for generated dynamic default catalogs so
DefaultDynamicBPMDetectionParameters no longer exposes Parameter<(), T> or requires a generated
DynamicBPMDetectionConfigAccessor for () bridge. Preserve DynamicBPMDetectionParameters<Config> as the config-bound
Parameter<Config, T> catalog. Do not migrate static BPM, normal distribution, or GUI config. Do not change runtime sync
behavior, serde schemas, labels, ranges, defaults, units, steps, logarithmic flags, or dynamic visitor order. Update
docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Metadata-Only Dynamic Parameter Specs

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none.

### Files changed

- `rust/crates/parameter/src/lib.rs`
- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/parameter_group.rs`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameter_adapters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Added `parameter::ParameterSpec<ValueType>` as the metadata-only shape for generated default parameter catalogs. The
dynamic `#[parameter_group]` macro now generates `DefaultDynamicBPMDetectionParameters` as a concrete spec catalog whose
associated constants are `ParameterSpec<T>`, while `DynamicBPMDetectionParameters<Config>` remains the config-bound
`Parameter<Config, T>` catalog.

### Behavioral changes

None intended.

Dynamic labels, ranges, defaults, units, steps, logarithmic flags, serde field shapes, validation order, plugin dynamic
remote-control order, and plugin dynamic host-read roundtrip behavior are preserved.

### Structural changes

- Removed generated `impl DynamicBPMDetectionConfigAccessor for ()` from the macro output.
- Replaced the generated `Default*Parameters = *Parameters<()>` alias with a generated concrete `Default*Parameters`
  struct containing `ParameterSpec<T>` constants.
- Kept the existing config-bound visitor pattern unchanged for real config catalogs.
- Moved dynamic plugin remote-control ordering and host reads from `DefaultDynamicBPMDetectionParameters::visit` to
  `DynamicBPMDetectionParameters<DynamicBPMDetectionConfig>::visit`.
- Updated dynamic-only tests that previously asserted visitor behavior through `Config = ()`.

### Affected boundaries / integration points

- `parameter` owns the new metadata-only public type.
- `parameter_macros` generates specs and no longer generates fake dynamic accessors for `()`.
- `bpm_detection_core`, `gui`, and `midi-bpm-detector-plugin` continue consuming the generated dynamic public API.
- Static BPM, normal distribution, and GUI default aliases still use the hand-written `Parameters<()>` bridge by design.
- No egui, nih-plug, desktop, wasm, or plugin dependencies were added to `parameter`, `parameter_macros`, or
  `bpm_detection_core`.

### Tests / checks

- Red run: `cargo test -p parameter_macros --test parameter_group` failed because `parameter::ParameterSpec` did not yet
  exist.
- `cargo test -p parameter_macros --test parameter_group`: passed, 2 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p parameter`: passed, 0 tests.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: initially failed because a test still asserted `SlideAdder` visitor support for `Config = ()`;
  passed after updating that assertion to `DynamicBPMDetectionConfig`, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- `cargo +nightly fmt --all -- --check`: passed after applying rustfmt.
- From repo root, `git diff --check`: passed.

### Decisions made

- Used the name `ParameterSpec<ValueType>` to make the no-config-accessor shape explicit.
- Did not add a metadata/spec visitor in this slice. Dynamic plugin ordering and host reads use the existing
  config-bound visitor with `DynamicBPMDetectionConfig`.
- Did not add a `Parameter::from_spec` const helper because generic `ParameterSpec` destruction cannot be evaluated in a
  stable const fn; the macro emits spec constants and config-bound parameter constants from the same field metadata.

### Deviations from brief

None intentional.

### Remaining risks

- The generated macro output still duplicates metadata literals internally between spec constants and config-bound
  `Parameter` constants, although both are generated from the same field attributes.
- Static BPM, normal distribution, and GUI fake-config default aliases remain as known follow-up debt.
- Visitor methods still have default implementations, so visitor exhaustiveness remains unchanged from the previous
  slice.

### Recommended next slice

Coordinator review of the `ParameterSpec` API and generated macro output, then either migrate `NormalDistributionConfig`
or `GUIConfig` to the attribute macro once the metadata/spec split is accepted.

## Coordinator Review: Metadata-Only Dynamic Parameter Specs

### Status

Verified.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: included in the coordinator checkpoint commit that follows `a5fc659 Improve parameter macro diagnostics`.

### Review result

The implementation satisfies the slice brief. `ParameterSpec<ValueType>` now owns metadata-only declarations, and the
generated dynamic default catalog no longer exposes `Parameter<(), T>` or requires `DynamicBPMDetectionConfigAccessor for
()`. The config-bound dynamic catalog remains `DynamicBPMDetectionParameters<Config>`.

Plugin dynamic remote controls and dynamic host reads now traverse
`DynamicBPMDetectionParameters<DynamicBPMDetectionConfig>`, preserving ordering while avoiding the metadata-only catalog
for behavior.

Static BPM, normal distribution, and GUI fake-config aliases remain deliberately unchanged.

### Fresh verification

Run from `rust/` unless noted:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter`: passed, 0 tests.
- `cargo test -p parameter_macros`: passed, including 4 diagnostics tests and 2 parameter-group tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- From repo root, `git diff --check`: passed.

### Remaining risks

- The macro still emits duplicate metadata literals internally for specs and config-bound parameters, but both are
  generated from the same field attributes.
- Hand-written `Parameters<()>` bridges still exist for static BPM, normal distribution, and GUI config.
- Visitor methods still have default implementations.

### Recommended next slice

Apply the attribute macro to `NormalDistributionConfig` only. This is the smallest remaining plain core parameter group:
it has no static BPM computed methods and does not require adding the macro dependency to `gui`.

## Slice Brief: Attribute Macro For NormalDistributionConfig

### Objective

Apply the existing generic `#[parameter_group(...)]` macro to `NormalDistributionConfig` only.

Use ordinary Rust struct fields plus small `#[parameter(...)]` metadata as the source of truth, and generate the normal
distribution accessor trait, concrete accessor impl, metadata spec catalog, config-bound parameter catalog, default impl,
validation, and traversal.

### Non-goals

- Do not migrate `GUIConfig`.
- Do not migrate `StaticBPMDetectionConfig`.
- Do not change static BPM computed methods such as `index_to_bpm`, `highest_bpm`, or `lowest_bpm`.
- Do not change runtime sync behavior in desktop, wasm, or plugin code.
- Do not change serde schemas, public config fields, labels, ranges, defaults, units, steps, logarithmic flags, or plugin
  parameter IDs.
- Do not replace or tighten the visitor pattern globally.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the metadata-spec coordinator review.
- `docs/audits/parameter-flow/handoff.md`, especially the dynamic macro and metadata-spec back-handoffs.
- `docs/parameter-flow-audit.md`, especially the normal distribution inventory.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/parameter_group.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Relevant boundaries / integration points

- `bpm_detection_core` already depends on `parameter_macros`; do not add egui, nih-plug, desktop, wasm, or plugin
  dependencies to core or the macro crate.
- Normal distribution plugin host parameters are manually enumerated under the static parameter group. Preserve those
  fields and IDs unless this slice explicitly changes only their source parameter constants.
- Static BPM owns the nested `NormalDistributionConfig`; do not change static model computed-method semantics.
- Keep shipped TOML defaults and serde field names stable.

### Expected behavioral change

None intended.

This is a structural refactor of normal distribution parameter declarations only.

### Expected structural change

- Annotate `NormalDistributionConfig` with `#[parameter_group(...)]`.
- Move normal distribution labels, ranges, units, steps, logarithmic flags, and defaults into field-level
  `#[parameter(...)]` metadata.
- Remove the hand-written normal distribution accessor trait, fake `impl ... for ()`, concrete accessor impl, default
  alias, parameter catalog, default impl, and validation impl, replacing them with generated equivalents.
- Add or update normal distribution traversal/inventory tests if the macro now generates a visitor for the group.

### Acceptance criteria

- `NormalDistributionConfig` remains an ordinary Rust struct with real fields.
- `DefaultNormalDistributionParameters::*` exposes `ParameterSpec<T>`, not `Parameter<(), T>`.
- There is no `impl NormalDistributionConfigAccessor for ()`.
- `NormalDistributionParameters<Config>` remains available as the config-bound catalog.
- Existing normal distribution labels, units, ranges, steps, logarithmic flags, defaults, serde field names, and plugin
  parameter IDs are preserved.
- Static BPM, dynamic config, and GUI config are not migrated in this slice.
- Plugin tests still confirm normal distribution parameter exposure/initialization where currently covered.

### Tests / checks

- From `rust/`: `cargo test -p parameter_macros`
- From `rust/`: `cargo test -p bpm_detection_core`
- From `rust/`: `cargo test -p gui`
- From `rust/`: `cargo test -p midi-bpm-detector-plugin`
- From `rust/`:
  `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`
- From `rust/`: `cargo +nightly fmt --all -- --check`
- From repo root: `git diff --check`

### Risks / open questions

- Generated visitors for normal distribution may reveal whether the dynamic visitor shape is too domain-specific. Keep
  changes minimal and preserve the generated API if it compiles cleanly.
- Plugin normal distribution controls are still manually enumerated. Do not force visitor-driven plugin construction in
  this slice unless it falls out naturally and stays small.
- Static BPM still has computed methods and remains the harder macro migration.

### Back-handoff requirements

Update `docs/audits/parameter-flow/handoff.md` with:

- status: complete / partial / blocked;
- branch and commit if applicable;
- files changed;
- summary of the generated normal distribution API shape;
- tests/checks run and results;
- deviations from this brief;
- remaining risks;
- recommended next slice.

## Prompt For Fresh Implementer Chat: Attribute Macro For NormalDistributionConfig

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Attribute Macro For NormalDistributionConfig" from docs/audits/parameter-flow/handoff.md.

Apply the existing generic #[parameter_group(...)] macro to NormalDistributionConfig only. Preserve serde schemas, public
config fields, labels, ranges, defaults, units, steps, logarithmic flags, plugin parameter IDs, and runtime sync behavior.
Do not migrate GUIConfig or StaticBPMDetectionConfig. Do not change static BPM computed methods. Update
docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Attribute Macro For NormalDistributionConfig

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none at implementer handoff time; included in the coordinator checkpoint commit with the metadata-spec slice.

### Files changed

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Applied the existing generic `#[parameter_group(...)]` macro to `NormalDistributionConfig` only. The normal distribution
config remains an ordinary Rust struct with real public fields, serde derives, `deny_unknown_fields`, and derivative
float comparisons. Field-level `#[parameter(...)]` metadata now owns the existing normal distribution labels, ranges,
defaults, units, and logarithmic flags.

The generated public normal distribution API now matches the current macro shape:

- `NormalDistributionConfigAccessor`
- `impl NormalDistributionConfigAccessor for NormalDistributionConfig`
- `DefaultNormalDistributionParameters` with `ParameterSpec<T>` associated constants
- `NormalDistributionParameters<Config>` with config-bound `Parameter<Config, T>` constants
- `NormalDistributionParameterVisitor<Config>`
- `NormalDistributionParameters::<Config>::visit`
- `NormalDistributionConfig::default`
- `NormalDistributionConfig::validate`

### Behavioral changes

None intended.

Normal distribution config fields, serde field names, validation ranges, labels, defaults, units, steps, logarithmic
flags, plugin host parameter construction, plugin parameter IDs, GUI slider listing, static update routing, and runtime
sync behavior are preserved.

### Structural changes

- Removed the hand-written normal distribution accessor trait, fake `impl NormalDistributionConfigAccessor for ()`,
  concrete accessor impl, `DefaultNormalDistributionParameters = NormalDistributionParameters<()>` alias, parameter
  catalog, default impl, and validation impl from source.
- Replaced them with macro-generated equivalents from the annotated `NormalDistributionConfig`.
- Added a core inventory test asserting that `DefaultNormalDistributionParameters::*` are `ParameterSpec<T>` values and
  that generated normal traversal preserves the field-source order.

### Affected boundaries / integration points

- `bpm_detection_core` continues to expose the normal distribution accessor and config-bound parameter catalog consumed
  by `gui`, `desktop`, `wasm`, and `midi-bpm-detector-plugin`.
- Plugin normal distribution host params remain manually enumerated under the static parameter group and continue to use
  the same generated `NormalDistributionParameters<NormalDistributionConfig>` constants.
- Static BPM still owns the nested `NormalDistributionConfig`; static computed methods were not changed.
- No egui, nih-plug, desktop, wasm, plugin, or new dependency surface was added to core or the macro crate.

### Tests / checks

- Red run: `cargo test -p bpm_detection_core parameter_inventory_tests` failed because
  `NormalDistributionParameterVisitor` was not yet generated.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 2 tests.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 4 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- `cargo +nightly fmt --all -- --check`: initially reported formatting changes in the new core test; passed after
  running `cargo +nightly fmt --all`.
- From repo root, `git diff --check`: passed.

### Decisions made

- Kept GUI and plugin normal distribution enumeration manual in this slice, because the brief only required the
  declaration/source-of-truth migration.
- Used the normal struct field order for generated `visit`: `std_dev`, `factor`, `cutoff`, `resolution`.
- Kept field comments for cutoff and resolution in place beside the ordinary Rust fields.

### Deviations from brief

None intentional.

### Remaining risks

- Normal distribution now has a generated visitor, but current GUI/plugin call sites do not use it yet.
- Static BPM and GUI still have hand-written `Parameters<()>` fake-config aliases.
- Visitor methods still have default implementations, so exhaustiveness remains a later audit concern.

### Recommended next slice

Coordinator review of the normal migration and generated visitor shape, then migrate `GUIConfig` if the macro output and
metadata-spec shape are accepted. Static BPM should remain later because its computed methods still need an explicit split
design.

## Coordinator Review: Attribute Macro For NormalDistributionConfig

### Status

Verified.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: included in the coordinator checkpoint commit with the metadata-spec and normal distribution slices.

### Review result

The implementation satisfies the slice brief. `NormalDistributionConfig` remains an ordinary Rust struct, normal
distribution metadata is field-local, `DefaultNormalDistributionParameters` exposes `ParameterSpec<T>` values, and
`NormalDistributionParameters<Config>` remains the config-bound catalog.

The generated `NormalDistributionParameterVisitor<Config>` preserves struct field order:

1. `std_dev`
2. `factor`
3. `cutoff`
4. `resolution`

Plugin normal distribution parameters remain manually enumerated under the static parameter group, so this slice did not
broaden runtime behavior or host parameter IDs.

### Fresh verification

Run from `rust/` unless noted:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 2 tests.
- `cargo test -p bpm_detection_core`: passed, 4 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- From repo root, `git diff --check`: passed.

### Remaining risks

- GUI config still has the hand-written `DefaultGUIParameters = GUIParameters<()>` fake-config alias.
- Static BPM still has the hand-written fake-config alias and computed accessor methods.
- Generated visitors still have default methods, so visitor exhaustiveness remains a later audit concern.

### Recommended next slice

Apply the attribute macro to `GUIConfig` only. Keep the current GUI/display runtime update behavior unchanged, including
the existing desktop/wasm/plugin propagation paths.

## Slice Brief: Attribute Macro For GUIConfig

### Objective

Apply the existing generic `#[parameter_group(...)]` macro to `GUIConfig` only.

Use ordinary Rust struct fields plus small `#[parameter(...)]` metadata as the source of truth, and generate the GUI
config accessor trait, concrete accessor impl, metadata spec catalog, config-bound parameter catalog, default impl,
validation, and traversal.

### Non-goals

- Do not migrate `StaticBPMDetectionConfig`.
- Do not change GUI/display runtime update semantics in desktop, wasm, or plugin code.
- Do not change interpolation behavior.
- Do not change serde schemas, public config fields, labels, ranges, defaults, units, steps, or logarithmic flags.
- Do not refactor the settings panel layout or plugin GUI/display host parameters beyond what is required to preserve
  current API compatibility.
- Do not replace or tighten the visitor pattern globally.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the metadata-spec and normal-distribution coordinator reviews.
- `docs/audits/parameter-flow/handoff.md`, especially the dynamic, metadata-spec, and normal-distribution back-handoffs.
- `docs/parameter-flow-audit.md`, especially the GUI/display inventory and update-path notes.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/gui/Cargo.toml`
- `rust/crates/gui/src/config.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Relevant boundaries / integration points

- The `gui` crate may need a dependency on `parameter_macros`; do not introduce dependencies on plugin, desktop, wasm, or
  MIDI crates.
- Desktop, wasm, and plugin runtime propagation for GUI/display parameters must remain unchanged.
- Plugin GUI/display host parameter IDs must remain `interpolation_duration` and `interpolation_curve`.
- GUI/display parameters are still semantically display/interpolation settings, not dynamic BPM scoring settings.

### Expected behavioral change

None intended.

This is a structural refactor of GUI/display parameter declarations only.

### Expected structural change

- Annotate `GUIConfig` with `#[parameter_group(...)]`.
- Move GUI labels, ranges, units, steps, logarithmic flags, and defaults into field-level `#[parameter(...)]` metadata.
- Remove the hand-written GUI accessor trait, fake `impl GUIConfigAccessor for ()`, concrete accessor impl, default alias,
  parameter catalog, default impl, and validation impl, replacing them with generated equivalents.
- Add or update GUI inventory/traversal tests if the macro now generates a visitor for the group.

### Acceptance criteria

- `GUIConfig` remains an ordinary Rust struct with real fields.
- `DefaultGUIParameters::*` exposes `ParameterSpec<T>`, not `Parameter<(), T>`.
- There is no `impl GUIConfigAccessor for ()`.
- `GUIParameters<Config>` remains available as the config-bound catalog.
- Existing GUI labels, units, ranges, steps, logarithmic flags, defaults, serde field names, and plugin host parameter IDs
  are preserved.
- Desktop, wasm, and plugin GUI/display update routing is not changed.
- Static BPM is not migrated in this slice.

### Tests / checks

- From `rust/`: `cargo test -p parameter_macros`
- From `rust/`: `cargo test -p gui`
- From `rust/`: `cargo test -p midi-bpm-detector-plugin`
- From `rust/`: `cargo test -p desktop`
- From `rust/`: `cargo test -p wasm --target wasm32-unknown-unknown` if the local wasm target is installed; otherwise
  record the setup blocker in the back-handoff.
- From `rust/`:
  `cargo clippy -p parameter_macros -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`
- From `rust/`: `cargo +nightly fmt --all -- --check`
- From repo root: `git diff --check`

### Risks / open questions

- `GUIConfig` lives outside core, so this slice will prove whether the macro API is usable from another crate without
  leaking core assumptions.
- The settings panel currently lists GUI parameters manually. Keep any visitor adoption narrow and do not redesign the UI
  in this slice.
- Wasm target availability may vary locally; record the exact blocker if that check cannot run.

### Back-handoff requirements

Update `docs/audits/parameter-flow/handoff.md` with:

- status: complete / partial / blocked;
- branch and commit if applicable;
- files changed;
- summary of the generated GUI API shape;
- tests/checks run and results;
- deviations from this brief;
- remaining risks;
- recommended next slice.

## Prompt For Fresh Implementer Chat: Attribute Macro For GUIConfig

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Attribute Macro For GUIConfig" from docs/audits/parameter-flow/handoff.md.

Apply the existing generic #[parameter_group(...)] macro to GUIConfig only. Preserve serde schemas, public config fields,
labels, ranges, defaults, units, steps, logarithmic flags, plugin host parameter IDs, interpolation behavior, and current
desktop/wasm/plugin GUI-display update routing. Do not migrate StaticBPMDetectionConfig. Update
docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Attribute Macro For GUIConfig

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none at implementer handoff time.

### Files changed

- `rust/Cargo.lock`
- `rust/crates/gui/Cargo.toml`
- `rust/crates/gui/src/config.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Applied the existing generic `#[parameter_group(...)]` macro to `GUIConfig` only. The GUI config remains an ordinary Rust
struct with real public fields, serde derives, `deny_unknown_fields`, and the existing interpolation field comments.
Field-level `#[parameter(...)]` metadata now owns the existing GUI labels, ranges, defaults, units, steps, and
logarithmic flags.

The generated public GUI API now matches the current macro shape:

- `GUIConfigAccessor`
- `impl GUIConfigAccessor for GUIConfig`
- `DefaultGUIParameters` with `ParameterSpec<T>` associated constants
- `GUIParameters<Config>` with config-bound `Parameter<Config, T>` constants
- `GUIParameterVisitor<Config>`
- `GUIParameters::<Config>::visit`
- `GUIConfig::default`
- `GUIConfig::validate`

### Behavioral changes

None intended.

GUI config fields, serde field names, validation ranges, labels, defaults, units, steps, logarithmic flags, plugin host
parameter IDs, interpolation behavior, settings-panel ordering, and desktop/wasm/plugin GUI-display update routing are
preserved.

### Structural changes

- Added `parameter_macros` as a local dependency of the `gui` crate.
- Removed the hand-written GUI accessor trait, fake `impl GUIConfigAccessor for ()`, concrete accessor impl,
  `DefaultGUIParameters = GUIParameters<()>` alias, parameter catalog, default impl, and validation impl from source.
- Replaced them with macro-generated equivalents from the annotated `GUIConfig`.
- Added a GUI inventory test asserting that `DefaultGUIParameters::*` are `ParameterSpec<T>` values and that generated GUI
  traversal preserves field-source order.

### Affected boundaries / integration points

- `gui` now depends on the local `parameter_macros` proc-macro crate.
- Desktop, wasm, and plugin runtime wrappers continue to implement the same `GUIConfigAccessor` methods.
- Plugin GUI/display host parameters still use IDs `interpolation_duration` and `interpolation_curve`.
- No plugin, desktop, wasm, MIDI, or `nih-plug` dependencies were added to `gui`, `parameter`, or `parameter_macros`.
- Static BPM remains hand-written and was not touched.

### Tests / checks

- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo +nightly fmt --all -- --check`: passed.
- `cargo clippy -p parameter_macros -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- From repo root, `git diff --check`: passed.

### Decisions made

- Named the generated visitor `GUIParameterVisitor` and kept current GUI call sites manually enumerated, because the
  brief only required the declaration/source-of-truth migration.
- Used the GUI struct field order for generated `visit`: `interpolation_duration`, then `interpolation_curve`.
- Kept settings-panel ordering unchanged by leaving `config_ui.rs` untouched.

### Deviations from brief

None intentional.

### Remaining risks

- GUI now has a generated visitor, but current GUI/plugin call sites do not use it yet.
- Static BPM still has the hand-written fake-config alias and computed accessor methods.
- Generated visitor methods still have default implementations, so visitor exhaustiveness remains a later audit concern.

### Recommended next slice

Coordinator review of the GUI migration and cross-crate macro dependency shape, then design the static BPM computed-method
split before applying the macro to `StaticBPMDetectionConfig`.

## Coordinator Review: Attribute Macro For GUIConfig

Review status: accepted.

The GUI migration matches the brief:

- `GUIConfig` remains an ordinary Rust struct with real public fields, serde derives, `deny_unknown_fields`, and the
  existing interpolation comments.
- GUI parameter metadata now lives in field-level `#[parameter(...)]` attributes.
- `DefaultGUIParameters` is now a generated `ParameterSpec<T>` metadata catalog rather than a
  `GUIParameters<()>` alias.
- The fake `impl GUIConfigAccessor for ()` bridge is gone.
- `GUIParameters<Config>` remains the config-bound parameter catalog.
- Desktop, wasm, and plugin runtime wrappers continue to implement `GUIConfigAccessor`.
- Plugin GUI/display host parameter IDs remain `interpolation_duration` and `interpolation_curve`.
- GUI/display update routing remains unchanged.
- `StaticBPMDetectionConfig` remains hand-written.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo clippy -p parameter_macros -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- From repo root, `git diff --check`: passed.

Coordinator judgment: all plain typed parameter groups now use the generic attribute macro. Static BPM remains the final
typed group outside the macro path because its accessor trait mixes parameter field access with computed methods:
`index_to_bpm`, `highest_bpm`, and `lowest_bpm`. Split that method family out before applying the macro to
`StaticBPMDetectionConfig`.

## Slice Brief: Static BPM Computed-Method Split

This slice is complete. The brief is retained as historical context for the back-handoff below.

### Objective

Make `StaticBPMDetectionConfigAccessor` macro-ready by removing non-field computed methods from its contract.

After this slice, `StaticBPMDetectionConfigAccessor` should represent only the static parameter fields that map to typed
parameters:

- `bpm_center`
- `bpm_range`
- `sample_rate`
- their setters

Move `index_to_bpm`, `highest_bpm`, and `lowest_bpm` behind a separate computed-method trait or equivalent extension
boundary so GUI histogram code and runtime wrapper code can keep calling those methods without every wrapper manually
implementing them.

Do not apply `#[parameter_group(...)]` to `StaticBPMDetectionConfig` in this slice.

### Non-goals

- Do not migrate `StaticBPMDetectionConfig` to the attribute macro yet.
- Do not change static BPM formulas, validation behavior, labels, ranges, defaults, units, steps, logarithmic flags, serde
  field names, plugin host parameter IDs, GUI histogram rendering, or runtime update routing.
- Do not change `NormalDistributionConfig` ownership inside `StaticBPMDetectionConfig`.
- Do not broaden this into plugin host mapping cleanup, GUI/display update routing, dynamic task overload cleanup, or
  parameter-like atomic state.
- Do not reintroduce a group-specific `macro_rules!` parameter DSL.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the GUI coordinator review and recommended slice sequence.
- `docs/audits/parameter-flow/repo-map.md`, especially the static BPM and GUI call-site notes.
- `docs/parameter-flow-audit.md`, especially the typed parameter inventory and invariant audit.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/application_parameters.rs`
- `rust/crates/gui/src/app.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/desktop/src/live_parameters.rs`
- `rust/crates/wasm/src/lib.rs`
- `rust/crates/midi-bpm-detector-plugin/src/bpm_detector_configuration.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameter_adapters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `docs/audits/parameter-flow/handoff.md` for the required back-handoff

### Relevant boundaries / integration points

- `StaticBPMDetectionParameters<Config>` should remain the config-bound static parameter catalog.
- If this slice replaces `DefaultStaticBPMDetectionParameters = StaticBPMDetectionParameters<()>`, keep the replacement
  narrow by using `ParameterSpec<T>` and preserve existing default metadata exactly.
- GUI plotting currently calls `index_to_bpm`, `lowest_bpm`, and `highest_bpm` through the shared application-parameter
  bound.
- Desktop, wasm, and plugin wrappers currently implement static field accessors and computed methods by delegating to
  their nested `StaticBPMDetectionConfig`.
- Plugin host parameter construction reads static field parameters from `StaticBPMDetectionParameters`.
- `NormalDistributionConfig` is nested under static config but already uses the macro. Do not merge it into the static
  field group.

### Expected behavioral change

None intended.

This is a contract-shaping refactor only. The computed methods should return the same values for the same configs, and
the GUI/plugin/desktop/wasm update flows should behave the same.

### Expected structural change

- Introduce a separate computed-method trait or extension boundary for `index_to_bpm`, `highest_bpm`, and `lowest_bpm`.
- Keep `StaticBPMDetectionConfigAccessor` focused on the three static parameter fields and setters.
- Remove the manual computed-method implementations from desktop, wasm, and plugin wrapper accessor impls if a blanket
  extension trait makes them redundant.
- Prefer replacing the remaining static fake-config default catalog with `ParameterSpec<T>` if it stays within this
  narrow slice; otherwise document why that is deferred to the macro-migration slice.
- Add or update focused tests proving static parameter defaults/specs and computed methods are preserved.

### Acceptance criteria

- `StaticBPMDetectionConfigAccessor` no longer requires `index_to_bpm`, `highest_bpm`, or `lowest_bpm`.
- GUI histogram code and shared application parameters can still call the computed methods through an explicit trait
  bound/import.
- Desktop, wasm, and plugin static wrappers still compile and propagate static config changes as before.
- Static parameter labels, ranges, defaults, units, steps, logarithmic flags, and plugin host parameter IDs are unchanged.
- No `#[parameter_group(...)]` annotation is added to `StaticBPMDetectionConfig`.
- No new runtime synchronization behavior is introduced.
- The implementer back-handoff records whether the static fake-config alias was removed or intentionally deferred.

### Suggested checks

From `rust/`:

```sh
cargo +nightly fmt --all -- --check
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p bpm_detection_core
cargo test -p gui
cargo test -p midi-bpm-detector-plugin
cargo test -p desktop
cargo test -p wasm --target wasm32-unknown-unknown
cargo clippy -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings
```

From repo root:

```sh
git diff --check
```

## Historical Prompt For Static Split Implementer

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Static BPM Computed-Method Split" from docs/audits/parameter-flow/handoff.md.

Split `index_to_bpm`, `highest_bpm`, and `lowest_bpm` away from `StaticBPMDetectionConfigAccessor` so that accessor
contract contains only the static parameter fields (`bpm_center`, `bpm_range`, `sample_rate`) and setters. Preserve all
static parameter metadata, defaults, validation behavior, serde field names, plugin host parameter IDs, GUI histogram
behavior, and desktop/wasm/plugin runtime update routing. Do not apply `#[parameter_group(...)]` to
`StaticBPMDetectionConfig` yet. Update docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Static BPM Computed-Method Split

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none; changes are currently uncommitted.

### Files changed

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/application_parameters.rs`
- `rust/crates/desktop/src/live_parameters.rs`
- `rust/crates/wasm/src/lib.rs`
- `rust/crates/midi-bpm-detector-plugin/src/bpm_detector_configuration.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Split the static BPM computed methods from `StaticBPMDetectionConfigAccessor`. The accessor trait now contains only
`bpm_center`, `bpm_range`, `sample_rate`, and their setters. Added `StaticBPMDetectionComputed` as the explicit computed
method extension boundary for `index_to_bpm`, `highest_bpm`, and `lowest_bpm`, with a blanket implementation for static
field accessors.

### Behavioral changes

No intended runtime behavior changes. Static BPM formulas, validation, serde field names, plugin host parameter IDs, GUI
histogram behavior, and desktop/wasm/plugin update routing are preserved.

### Structural changes

- Added `StaticBPMDetectionComputed`.
- Made `BPMDetectionConfig` require `StaticBPMDetectionComputed` so GUI histogram callers still access computed methods
  through an explicit bound.
- Removed manual computed-method delegations from desktop, wasm, and plugin static wrapper accessor impls.
- Replaced `DefaultStaticBPMDetectionParameters = StaticBPMDetectionParameters<()>` with a concrete
  `DefaultStaticBPMDetectionParameters` metadata catalog using `ParameterSpec<T>`.
- Removed the fake `impl StaticBPMDetectionConfigAccessor for ()` bridge.
- Added focused static parameter inventory and computed-method extension tests.

### Affected boundaries / integration points

- `bpm_detection_core` public parameter contracts changed by splitting static field access from computed methods.
- Shared `gui::BPMDetectionConfig` now carries the explicit computed-method bound used by histogram rendering.
- Desktop, wasm, and plugin static wrapper implementations still own field mutation and propagation behavior.
- Plugin host parameter construction still reads static field parameters from
  `StaticBPMDetectionParameters<StaticBPMDetectionConfig>`.

### Tests / checks

- From `rust/`, `cargo +nightly fmt --all -- --check`: passed.
- From `rust/`, `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 4 tests.
- From `rust/`, `cargo test -p bpm_detection_core`: passed, 6 tests.
- From `rust/`, `cargo test -p gui`: passed, 2 tests.
- From `rust/`, `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- From `rust/`, `cargo test -p desktop`: passed, 13 tests.
- From `rust/`, `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- From `rust/`,
  `cargo clippy -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- From repo root, `git diff --check`: passed.

### Decisions made

- Removed the static fake-config alias and `()` accessor bridge in this slice because the `ParameterSpec<T>` replacement
  stayed narrow and matched the already accepted generated default-catalog shape.
- Kept `StaticBPMDetectionConfig` hand-written and did not add `#[parameter_group(...)]`.
- Kept the computed extension trait focused on only `index_to_bpm`, `highest_bpm`, and `lowest_bpm`.

### Deviations from brief

None.

### Remaining risks

- External downstream code that directly used the artificial `StaticBPMDetectionParameters<()>` shape will no longer
  compile. No in-repo callers depended on it, and the public default catalog remains available as metadata specs.
- Static BPM still remains hand-written; macro migration is intentionally left for the next slice.

### Recommended next slice

Coordinator review of the static computed-method split and public API shape, then apply the existing
`#[parameter_group(...)]` macro pattern to `StaticBPMDetectionConfig` if the split is accepted.

## Coordinator Review: Static BPM Computed-Method Split

Review status: accepted.

Review checkpoint: `codex/parameter-flow-audit` at `bdab497 Stabilize parameter macro diagnostic tests`.

The static split matches the brief:

- `StaticBPMDetectionConfigAccessor` contains only `bpm_center`, `bpm_range`, `sample_rate`, and their setters.
- `StaticBPMDetectionComputed` is the explicit computed-method extension for `index_to_bpm`, `highest_bpm`, and
  `lowest_bpm`.
- `StaticBPMDetectionConfig` keeps inherent computed methods by delegating through the computed extension.
- Desktop, wasm, and plugin wrapper impls no longer hand-write computed-method delegations.
- `DefaultStaticBPMDetectionParameters` is now a concrete `ParameterSpec<T>` metadata catalog.
- The static fake `impl StaticBPMDetectionConfigAccessor for ()` bridge is gone.
- `StaticBPMDetectionConfig` remains hand-written and is not yet annotated with `#[parameter_group(...)]`.
- The CI follow-up commit only disables compiler color output for macro diagnostic fixture matching.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 4 tests.
- `cargo test -p bpm_detection_core`: passed, 6 tests.
- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo clippy -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- From repo root, `git diff --check`: passed.

GitHub PR state during review:

- Draft PR: <https://github.com/valsteen/midi_bpm_detection/pull/20>.
- CI observed green for `extension`, `format`, `native (aarch64-apple-darwin)`,
  `native (x86_64-unknown-linux-gnu)`, and `wasm`.
- `native (x86_64-apple-darwin)` was still in progress at the latest coordinator check.

Coordinator judgment: the split is accepted. Static BPM is now structurally aligned with the generated groups. The next
bounded slice should apply the existing `#[parameter_group(...)]` macro to `StaticBPMDetectionConfig`, while keeping
`StaticBPMDetectionComputed`, the inherent computed methods, and nested `NormalDistributionConfig` behavior intact.

## Slice Brief: Attribute Macro For StaticBPMDetectionConfig

This slice is complete. The brief is retained as historical context for the back-handoff below.

### Objective

Apply the existing generic `#[parameter_group(...)]` macro to `StaticBPMDetectionConfig`.

The generated static parameter group should cover only the real static BPM parameter fields:

- `bpm_center`
- `bpm_range`
- `sample_rate`

Keep `normal_distribution: NormalDistributionConfig` as a nested config field outside the generated static parameter
field group. Keep `StaticBPMDetectionComputed` and the inherent computed methods (`index_to_bpm`, `highest_bpm`,
`lowest_bpm`) as explicit non-generated behavior.

### Non-goals

- Do not include `normal_distribution` in `StaticBPMDetectionParameters`.
- Do not change static BPM formulas, validation behavior, labels, ranges, defaults, units, steps, logarithmic flags, serde
  field names, plugin host parameter IDs, GUI histogram rendering, or runtime update routing.
- Do not change the already generated `NormalDistributionConfig`, dynamic config, or GUI config macro shapes.
- Do not broaden this into GUI/plugin host mapping cleanup, GUI/display update routing, dynamic task overload cleanup, or
  parameter-like atomic state.
- Do not reintroduce a group-specific `macro_rules!` parameter DSL.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the static split coordinator review and recommended slice sequence.
- `docs/audits/parameter-flow/repo-map.md`, especially the static BPM, normal distribution, GUI, and plugin notes.
- `docs/parameter-flow-audit.md`, especially the typed parameter inventory and invariant audit.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/bpm_detection_core/src/parameters.rs`
- Static parameter inventory tests in `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameter_adapters.rs`
- `rust/crates/desktop/src/live_parameters.rs`
- `rust/crates/wasm/src/lib.rs`
- `docs/audits/parameter-flow/handoff.md` for the required back-handoff

### Relevant boundaries / integration points

- `bpm_detection_core` already depends on `parameter_macros`; no new cross-crate macro dependency is expected.
- `StaticBPMDetectionConfigAccessor` is now a pure field accessor contract and should be generated by the macro.
- `StaticBPMDetectionComputed` depends on the static field accessor contract and should continue to work with the
  generated accessor trait.
- `DefaultStaticBPMDetectionParameters` should remain a `ParameterSpec<T>` metadata catalog.
- `StaticBPMDetectionParameters<Config>` should remain the config-bound static parameter catalog consumed by GUI and
  plugin code.
- Plugin host parameter IDs for static fields are owned in plugin code and must remain unchanged:
  `bpm_center`, `bpm_range`, and `sample_rate`.
- `NormalDistributionConfig` is nested under static config but is already its own generated parameter group.

### Expected behavioral change

None intended.

This is the final declaration/source-of-truth migration for typed parameter groups. The macro-generated static code should
be behaviorally equivalent to the current hand-written static field accessor/catalog/default/validation machinery.

### Expected structural change

- Annotate `StaticBPMDetectionConfig` with `#[parameter_group(...)]`.
- Move static field metadata for `bpm_center`, `bpm_range`, and `sample_rate` into field-level `#[parameter(...)]`
  attributes.
- Remove hand-written static field accessor trait/impl, static parameter catalog, default metadata catalog, `Default`
  impl, and validation for those fields where the macro now generates equivalents.
- Keep or re-add the manual validation step for nested `normal_distribution.validate()`.
- Keep `StaticBPMDetectionComputed` and inherent computed methods outside the macro-generated area.
- Keep existing tests or add focused tests proving static specs, computed methods, and nested normal-distribution
  validation are preserved.

### Acceptance criteria

- `StaticBPMDetectionConfig` remains an ordinary Rust struct with real fields.
- `normal_distribution` remains a normal nested config field and is not a `Parameter` in `StaticBPMDetectionParameters`.
- `StaticBPMDetectionConfigAccessor`, `DefaultStaticBPMDetectionParameters`,
  `StaticBPMDetectionParameters<Config>`, and a static visitor type remain available under stable public names chosen in
  the group attribute.
- Static parameter labels, ranges, defaults, units, steps, logarithmic flags, and plugin host parameter IDs are unchanged.
- `StaticBPMDetectionComputed` and inherent computed methods still return the same values for representative configs.
- `StaticBPMDetectionConfig::validate()` still validates both static fields and nested normal distribution settings.
- No runtime synchronization behavior is introduced or changed.

### Tests / checks

From `rust/`:

```sh
cargo +nightly fmt --all -- --check
cargo test -p parameter_macros
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p bpm_detection_core
cargo test -p gui
cargo test -p midi-bpm-detector-plugin
cargo test -p desktop
cargo test -p wasm --target wasm32-unknown-unknown
cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings
```

From repo root:

```sh
git diff --check
```

### Risks / open questions

- The macro currently treats every annotated parameter field as part of generated defaulting and validation, while static
  config also has a nested non-parameter field. The implementer must preserve `normal_distribution` defaulting and
  validation explicitly if the macro-generated default/validate impls only cover annotated fields.
- Generated visitor defaults are still no-op methods, so visitor exhaustiveness remains a later audit concern.
- Downstream code outside this repo may notice the generated static visitor type if exported, but no in-repo callers
  should depend on a hand-written static visitor today.

### Back-handoff requirements

Record:

- files changed;
- generated public static API names;
- whether nested `normal_distribution` defaulting and validation stayed manual or required macro support;
- how static field metadata was proven unchanged;
- tests/checks run with pass/fail status;
- any deviations from this brief;
- recommended next slice after static macro migration.

## Historical Prompt For Static Macro Implementer

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Attribute Macro For StaticBPMDetectionConfig" from
docs/audits/parameter-flow/handoff.md.

Apply the existing generic #[parameter_group(...)] macro to StaticBPMDetectionConfig. Include only the static BPM
parameter fields (`bpm_center`, `bpm_range`, `sample_rate`) in the generated parameter group. Keep
`normal_distribution` as a nested config field outside StaticBPMDetectionParameters, and preserve
StaticBPMDetectionComputed, inherent computed methods, static labels/ranges/defaults, serde fields, plugin host parameter
IDs, GUI histogram behavior, and desktop/wasm/plugin runtime update routing. Update
docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: Attribute Macro For StaticBPMDetectionConfig

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none.

### Files changed

- `rust/crates/parameter_macros/src/lib.rs`
- `rust/crates/parameter_macros/tests/parameter_group.rs`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Applied the existing generic `#[parameter_group(...)]` macro to `StaticBPMDetectionConfig`. The static config remains an
ordinary Rust struct, and only these static BPM fields are annotated as generated parameters:

- `bpm_center`
- `bpm_range`
- `sample_rate`

Generated public static API names:

- `StaticBPMDetectionConfigAccessor`
- `DefaultStaticBPMDetectionParameters`
- `StaticBPMDetectionParameters<Config>`
- `StaticBPMDetectionParameterVisitor<Config>`

### Behavioral changes

None intended.

Static labels, ranges, defaults, units, steps, logarithmic flags, validation behavior, computed helper results, serde
field names, plugin host parameter IDs, GUI histogram behavior, and runtime update routing are preserved.

### Structural changes

- Moved static field metadata onto the three static BPM fields as `#[parameter(...)]` attributes.
- Removed the hand-written static accessor trait/impl, default metadata catalog, config-bound parameter catalog,
  `Default` impl, and static-field validation from `parameters.rs`; those are now generated by `parameter_group`.
- Kept `StaticBPMDetectionComputed` and the inherent computed methods outside the macro-generated area.
- Extended the generic macro so unannotated fields are still retained in generated `Default` and `validate` impls.

### Affected boundaries / integration points

- `bpm_detection_core` continues to expose the same static field accessor, default spec catalog, and config-bound
  parameter catalog names.
- `normal_distribution` remains a nested `NormalDistributionConfig` field and is not included in
  `StaticBPMDetectionParameters::visit`.
- GUI and plugin static parameter consumers continue to use `StaticBPMDetectionParameters::<Config>` constants.
- Plugin host parameter IDs remain owned by plugin code and were covered by the existing plugin ID test.
- No runtime synchronization code changed in desktop, wasm, plugin, or GUI crates.

### Tests / checks

Red check before implementation:

- `cargo test -p bpm_detection_core parameter_inventory_tests`: failed with missing
  `StaticBPMDetectionParameterVisitor`, as expected.

Final checks from `rust/`:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 7 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 8 tests.
- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.

Final check from repo root:

- `git diff --check`: passed.

### Decisions made

- Added macro support for unannotated fields by generating `Default::default()` initialization and
  `self.<field>.validate()?` calls for those fields. This preserved `normal_distribution` defaulting and validation
  without making it a static `Parameter`.
- Added a macro-level regression test proving unannotated nested fields are defaulted, validated, and omitted from
  generated `visit` traversal.
- Added core tests proving the static visitor lists only the three static BPM fields and that
  `StaticBPMDetectionConfig::validate()` still checks nested normal-distribution settings.

### Deviations from brief

None intentional.

### Remaining risks

- The macro now expects unannotated fields on a parameter group to implement both `Default` and a compatible
  `validate() -> Result<(), String>` method. That matches the current static nested config need but should be kept in
  mind before adding unrelated unannotated fields to future parameter groups.
- Generated visitor methods still have default no-op implementations; visitor exhaustiveness remains a separate audit
  concern.

### Recommended next slice

Coordinator review of the now-homogeneous typed parameter groups, with special attention to whether the unannotated
nested-field macro behavior should be documented as the supported shape or narrowed before future parameter-model work.

## Coordinator Review: Attribute Macro For StaticBPMDetectionConfig

Review status: accepted.

The static macro migration matches the brief:

- `StaticBPMDetectionConfig` remains an ordinary Rust struct with real fields.
- Static field metadata now lives in `#[parameter(...)]` attributes on `bpm_center`, `bpm_range`, and `sample_rate`.
- `normal_distribution` remains a nested `NormalDistributionConfig` and is omitted from
  `StaticBPMDetectionParameters::visit`.
- `StaticBPMDetectionComputed` and the inherent computed methods remain outside the generated parameter group.
- `StaticBPMDetectionConfigAccessor`, `DefaultStaticBPMDetectionParameters`,
  `StaticBPMDetectionParameters<Config>`, and `StaticBPMDetectionParameterVisitor<Config>` are generated under the
  intended public names.
- Runtime synchronization code in desktop, wasm, plugin, and GUI crates was not changed.

Macro contract decision:

- Unannotated fields in a parameter group are now accepted as nested config fields.
- Generated `Default` initializes unannotated fields with `Default::default()`.
- Generated `validate()` calls `self.<field>.validate()?` for unannotated fields.
- Generated parameter traversal omits unannotated fields.
- This contract is accepted for nested config values such as `normal_distribution`, but should not be casually used for
  unrelated helper fields.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 7 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 8 tests.
- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- From repo root, `git diff --check`: passed.

Coordinator judgment: all typed parameter groups are now homogeneous at the declaration/catalog layer. The next bounded
implementation slice should start reducing remaining GUI mapping boilerplate, but only where generated order already
matches the current UI. Do not replace normal-distribution manual ordering yet: generated normal-distribution traversal
is `std_dev`, `factor`, `cutoff`, `resolution`, while current GUI settings order is `std_dev`, `resolution`, `cutoff`,
`factor`.

## Slice Brief: GUI Settings Visitor Adoption For Matching Groups

This slice is complete. The brief is retained as historical context for the back-handoff below.

### Objective

Use generated parameter visitors in the GUI settings panel for the groups whose generated traversal order already matches
the current settings-panel order:

- GUI/display parameters;
- static BPM parameters.

The slice should reduce manual parameter lists in `BPMDetectionGUI::settings_panel` without changing the visible order or
behavior of the settings panel.

### Non-goals

- Do not change normal-distribution settings order.
- Do not replace the normal-distribution manual `slide_adder.add(...)` calls in this slice.
- Do not change plugin remote-control pages or plugin parameter construction.
- Do not change runtime synchronization, config schemas, labels, ranges, defaults, units, steps, logarithmic flags, or
  plugin host parameter IDs.
- Do not change generated visitor defaults or visitor exhaustiveness.
- Do not add new lint exceptions.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the static macro review and recommended slice sequence.
- `docs/audits/parameter-flow/repo-map.md`, especially the current GUI/plugin ordering notes.
- `docs/parameter-flow-audit.md`, especially the typed parameter inventory and flow trace.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/gui/src/config.rs`
- `rust/crates/bpm_detection_core/src/parameters.rs`
- `docs/audits/parameter-flow/handoff.md` for the required back-handoff

### Relevant boundaries / integration points

- `SlideAdder` already implements `DynamicBPMDetectionParameterVisitor`.
- Add visitor implementations for `GUIParameterVisitor` and `StaticBPMDetectionParameterVisitor`.
- `GUIParameters::visit` order is `interpolation_duration`, then `interpolation_curve`, which matches current settings
  panel order.
- `StaticBPMDetectionParameters::visit` order is `bpm_center`, `bpm_range`, then `sample_rate`, which matches current
  settings-panel order.
- `NormalDistributionParameters::visit` order does not match current settings-panel order; leave it manual.

### Expected behavioral change

None intended.

The settings panel should show the same controls in the same order and continue to update the same config/runtime paths.

### Expected structural change

- Implement generated visitor traits for `SlideAdder` for GUI/display and static BPM groups.
- Replace manual GUI/display and static `slide_adder.add(...)` lists in `config_ui.rs` with
  `GUIParameters::visit(&mut slide_adder)` and `StaticBPMDetectionParameters::visit(&mut slide_adder)`.
- Keep the normal-distribution manual list exactly ordered as today.
- Add focused compile-time trait assertions or tests where useful to prove `SlideAdder` supports the added visitors.

### Acceptance criteria

- `config_ui.rs` uses generated traversal for GUI/display and static BPM groups.
- Normal-distribution controls remain manually listed in current order: `STD_DEV`, `RESOLUTION`, `CUTOFF`, `FACTOR`.
- Dynamic controls continue to use generated traversal.
- GUI tests pass and no plugin/desktop/wasm behavior changes are introduced.

### Tests / checks

From `rust/`:

```sh
cargo +nightly fmt --all -- --check
cargo test -p gui
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p bpm_detection_core
cargo test -p midi-bpm-detector-plugin
cargo clippy -p gui -p bpm_detection_core -p midi-bpm-detector-plugin --all-targets -- -D warnings
```

From repo root:

```sh
git diff --check
```

### Risks / open questions

- Replacing the normal-distribution list with `NormalDistributionParameters::visit` would change visible control order;
  do not do that in this slice.
- This slice does not address plugin remote-control ordering, which has its own current order and should be handled
  separately.

### Back-handoff requirements

Record:

- files changed;
- which visitor impls were added;
- exact settings-panel order after the change;
- tests/checks run with pass/fail status;
- any deviations from this brief;
- recommended next slice.

## Historical Prompt For GUI Settings Visitor Implementer

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "GUI Settings Visitor Adoption For Matching Groups" from
docs/audits/parameter-flow/handoff.md.

Add `SlideAdder` visitor implementations for GUI/display and static BPM parameter groups, then replace the matching
manual GUI/display and static settings-panel `slide_adder.add(...)` calls with `GUIParameters::visit(...)` and
`StaticBPMDetectionParameters::visit(...)`. Preserve current settings-panel order. Leave normal-distribution settings
manual because their current UI order differs from generated field order. Do not change plugin remote controls or runtime
synchronization. Update docs/audits/parameter-flow/handoff.md with a back-handoff.
```

## Back-Handoff: GUI Settings Visitor Adoption For Matching Groups

### Status

Complete.

### Branch / commit

Branch: `codex/parameter-flow-audit`.

Commit: none.

### Files changed

- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/gui/src/config_ui.rs`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Adopted generated parameter visitors in the shared egui settings panel for the groups whose generated traversal order
already matches the current UI order.

Added `SlideAdder` visitor implementations for:

- `GUIParameterVisitor<Config>`
- `StaticBPMDetectionParameterVisitor<Config>`

Both impls use the generated visitor trait's generic `parameter(...)` fallback to call `SlideAdder::add(...)` once per
visited plain slider parameter, instead of repeating one method body per field.

`BPMDetectionGUI::settings_panel` now uses:

- `GUIParameters::visit(&mut slide_adder)`
- `StaticBPMDetectionParameters::visit(&mut slide_adder)`

The exact settings-panel order after the change is preserved:

1. runtime-specific `desktop_controls`;
2. GUI/display generated traversal: `INTERPOLATION_DURATION`, `INTERPOLATION_CURVE`;
3. static BPM generated traversal: `BPM_CENTER`, `BPM_RANGE`, `SAMPLE_RATE`;
4. normal distribution manual order: `STD_DEV`, `RESOLUTION`, `CUTOFF`, `FACTOR`;
5. dynamic generated traversal;
6. `Send tempo`.

### Behavioral changes

None intended. The settings panel renders the same controls in the same order and continues to write through the same
typed `Parameter` getter/setter paths.

### Structural changes

- `SlideAdder` now supports the generated GUI/display and static BPM visitor traits in addition to the existing dynamic
  visitor trait.
- Plain slider rendering was relaxed to require only `Asf64`, allowing visitor impls whose generic fallback receives
  `Parameter<Config, ValueType>` with the same bound as the generated trait.
- The matching manual GUI/display and static parameter lists in `config_ui.rs` were replaced with generated traversal
  calls.
- Focused compile-time trait assertion tests were added for GUI/display and static visitor support.

### Affected boundaries / integration points

- Shared `gui` crate settings-panel rendering now depends on the generated visitor APIs for GUI/display and static BPM
  groups.
- `bpm_detection_core` static generated visitor API is consumed by `gui`.
- Plugin remote controls, plugin parameter construction, runtime synchronization, config schemas, labels, ranges,
  defaults, units, steps, logarithmic flags, and host parameter IDs were not changed.

### Tests / checks

All requested checks passed.

- From `rust/`: `cargo +nightly fmt --all -- --check`
- From `rust/`: `cargo test -p gui`
- From `rust/`: `cargo test -p bpm_detection_core parameter_inventory_tests`
- From `rust/`: `cargo test -p bpm_detection_core`
- From `rust/`: `cargo test -p midi-bpm-detector-plugin`
- From `rust/`: `cargo clippy -p gui -p bpm_detection_core -p midi-bpm-detector-plugin --all-targets -- -D warnings`
- From repo root: `git diff --check`

### Decisions made

- Kept normal distribution manual because generated traversal order is `STD_DEV`, `FACTOR`, `CUTOFF`, `RESOLUTION`,
  which does not match the current settings-panel order.
- Used the generated visitor trait's generic fallback for GUI/display and static BPM, because every parameter in those
  groups renders through the same plain slider path.
- Added compile-time trait assertions in `add_slider.rs` rather than UI snapshot tests because this slice only changes
  traversal source, not widget behavior.

### Deviations from brief

None.

### Remaining risks

- This slice does not reduce plugin remote-control or plugin parameter-construction manual lists.
- The generated visitor traits still have their existing default-method behavior; visitor exhaustiveness was explicitly
  out of scope.

### Recommended next slice

Have the audit coordinator decide whether the next bounded slice should target plugin remote-control/static parameter
manual ordering or host parameter construction/readback reuse, keeping normal-distribution order differences explicit.

## Coordinator Review: GUI Settings Visitor Adoption For Matching Groups

Review status: accepted.

The slice matches the brief:

- `BPMDetectionGUI::settings_panel` now uses generated traversal for GUI/display and static BPM groups.
- The visible settings-panel order is preserved:
  1. runtime-specific `desktop_controls`;
  2. GUI/display generated traversal: `INTERPOLATION_DURATION`, `INTERPOLATION_CURVE`;
  3. static BPM generated traversal: `BPM_CENTER`, `BPM_RANGE`, `SAMPLE_RATE`;
  4. normal distribution manual order: `STD_DEV`, `RESOLUTION`, `CUTOFF`, `FACTOR`;
  5. dynamic generated traversal;
  6. `Send tempo`.
- Normal-distribution settings remain manual because generated traversal order differs from current UI order.
- Plugin remote controls, plugin parameter construction, runtime synchronization, config schemas, labels, ranges,
  defaults, units, steps, logarithmic flags, and host parameter IDs were not changed.

Visitor consumer decision:

- The generated visitor traits already have a generic `parameter<ValueType: Asf64>(...)` fallback.
- Field-specific visitor methods delegate to that generic fallback by default.
- `SlideAdder` now uses that generic fallback for GUI/display and static BPM groups, where every visited parameter uses
  the same plain slider path.
- `SlideAdder` keeps explicit dynamic field methods because `beats_lookback` uses a plain slider while `OnOff<f32>`
  fields use `add_on_off`.
- This is the preferred DX shape before considering another macro: use the generated fallback for homogeneous consumers,
  and explicit field overrides for heterogeneous consumers.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p gui`: passed, 4 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 6 tests.
- `cargo test -p bpm_detection_core`: passed, 8 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p gui -p bpm_detection_core -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- From repo root, `git diff --check`: passed.

Coordinator judgment: the visitor fallback handles the exact homogeneous shape raised in review without another macro.
Do not add a visitor-impl macro yet. First audit the remaining visitor consumers and manual parameter lists so future
cleanup can distinguish truly homogeneous consumers from order-sensitive lists and host-handle mappings.

## Slice Brief: Visitor Consumer Homogeneity Audit

### Objective

Inventory the remaining visitor implementations and manual parameter lists across GUI and plugin code, then classify each
consumer by the smallest clear abstraction that fits it.

Use these classifications:

- homogeneous generic-fallback consumer;
- heterogeneous explicit-field visitor;
- order-sensitive manual list;
- future helper candidate;
- leave-alone bespoke runtime/host mapping.

### Non-goals

- Do not add a new macro.
- Do not change runtime behavior.
- Do not change settings-panel order, plugin remote-control order, plugin host parameter IDs, config schemas, labels,
  ranges, defaults, units, steps, or logarithmic flags.
- Do not replace normal-distribution manual ordering in this slice.
- Do not change visitor exhaustiveness/default-method policy.

### Durable context to read first

- `docs/audits/parameter-flow/audit.md`, especially the GUI settings visitor review and visitor consumer decision.
- `docs/audits/parameter-flow/repo-map.md`, especially GUI/plugin ordering notes.
- `docs/parameter-flow-audit.md`, especially the typed parameter inventory and flow trace.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameter_adapters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/bpm_detector_configuration.rs`
- `rust/crates/midi-bpm-detector-plugin/src/task_executor.rs`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/handoff.md`

### Relevant boundaries / integration points

- GUI settings-panel ordering is user-visible and must stay explicit.
- Plugin remote-control page ordering is host-visible and must stay explicit.
- Plugin host parameter construction maps generated parameter metadata to concrete `nih-plug` parameter handles.
- Dynamic visitor consumers are not all homogeneous: some fields are plain numeric parameters, while many are
  `OnOff<f32>`.
- Normal-distribution generated traversal order differs from current GUI and plugin ordering.

### Expected behavioral change

None. This is an audit/documentation slice.

### Expected structural change

- Update durable audit docs with a table or concise inventory of visitor consumers/manual parameter lists.
- Identify which, if any, remaining consumers are safe candidates for generated visitor traversal or generic fallback.
- Recommend the next bounded implementation slice with explicit non-goals and checks.

### Acceptance criteria

- The audit names each remaining visitor implementation and manual parameter list relevant to GUI/plugin parameter flow.
- Normal-distribution GUI/plugin ordering differences are recorded with exact current orders.
- The docs explicitly say where not to add more abstraction yet.
- The next recommended implementation slice is small and preserves ordering/runtime behavior.

### Tests / checks

From repo root:

```sh
git diff --check
```

Optional from `rust/` if code is inspected but not changed:

```sh
cargo test -p gui
cargo test -p midi-bpm-detector-plugin
```

### Risks / open questions

- A tempting macro over visitor impls could obscure field-to-host-handle mappings in plugin code.
- A generated traversal could silently change host-visible or user-visible parameter order if ordering is not made
  explicit first.

### Back-handoff requirements

Record:

- files/docs changed;
- visitor/manual-list inventory;
- classification decisions;
- recommended next implementation slice;
- checks run.

## Prompt For Fresh Bounded Implementer

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Visitor Consumer Homogeneity Audit" from docs/audits/parameter-flow/handoff.md.

Inventory remaining visitor implementations and manual parameter lists across GUI and plugin code. Classify each as
homogeneous generic-fallback, heterogeneous explicit-field, order-sensitive manual list, future helper candidate, or
leave-alone bespoke runtime/host mapping. Do not add a new macro or change runtime behavior in this slice. Keep
normal-distribution ordering differences explicit. Update docs/audits/parameter-flow/handoff.md with a back-handoff and
next recommended implementation slice.
```

## Back-Handoff: Visitor Consumer Homogeneity Audit

### Status

Complete.

### Branch / commit

- Branch: `codex/parameter-flow-audit`
- Commit: not created in this implementation turn

### Files changed

- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Inventoried the remaining visitor implementations and manual parameter lists that affect shared GUI settings and plugin
host/remote-control parameter flow. Classified each consumer/list as homogeneous generic fallback, heterogeneous
explicit-field visitor, order-sensitive manual list, future helper candidate, or leave-alone bespoke runtime/host
mapping.

### Behavioral changes

None. This was a documentation-only audit slice.

### Structural changes

- Added a `Visitor Consumer Homogeneity Audit` section to `docs/audits/parameter-flow/audit.md`.
- Added a condensed `Visitor Consumers And Manual Lists` section to `docs/audits/parameter-flow/repo-map.md`.
- Recorded this back-handoff in `docs/audits/parameter-flow/handoff.md`.

### Affected boundaries / integration points

- Shared egui settings order remains user-visible and intentionally unchanged.
- Plugin CLAP remote-control order remains host-visible and intentionally unchanged.
- Plugin host parameter construction remains explicit so `nih-plug` IDs, callbacks, nested groups, and field-to-handle
  mappings stay auditable.
- Plugin host/config synchronization remains explicit because it encodes static/dynamic timing and GUI refresh side
  effects.

### Tests / checks

- `git diff --check`: passed from repo root.
- Optional Cargo checks were not run because only audit docs changed.

### Decisions made

- Treat `SlideAdder` GUI and static visitor impls as already-homogeneous generic-fallback consumers.
- Keep `SlideAdder` dynamic, plugin dynamic remote-control, and plugin dynamic host-config-reader visitor impls explicit
  because their field behavior or host-handle mapping is heterogeneous.
- Keep normal-distribution GUI and plugin remote-control lists manual until the project chooses ordering semantics.
- Keep plugin host construction, GUI-origin setters, and host-origin copy-back as bespoke runtime/host mappings for now.

### Deviations from brief

None.

### Remaining risks

- The long audit table is documentation-only and may need future pruning if the coordinator wants a shorter public-facing
  summary.
- Normal-distribution order is still intentionally split: generated traversal is `std_dev`, `factor`, `cutoff`,
  `resolution`; GUI settings are `std_dev`, `resolution`, `cutoff`, `factor`; plugin remote controls are `resolution`,
  `factor`, `cutoff`, `std_dev`.
- Plugin host/config synchronization still has manual lists and should not be treated as solved by this inventory.

### Recommended next slice

Make normal-distribution ordering policy explicit before replacing any normal-distribution manual lists. Keep the slice
small: document or test the intended GUI order, plugin remote-control order, and generated traversal order without
changing runtime behavior, host parameter IDs, remote-control order, `TaskExecutor` copy-back logic, or visitor
macro/helper policy.

## Slice Brief: Normal Distribution Ordering Policy

### Objective

Make the normal-distribution ordering policy explicit before any future slice replaces manual normal-distribution lists
with generated traversal or helper APIs.

Pin the three currently different orders as intentional current behavior:

- generated `NormalDistributionParameters::visit` order: `std_dev`, `factor`, `cutoff`, `resolution`;
- shared GUI settings-panel order: `STD_DEV`, `RESOLUTION`, `CUTOFF`, `FACTOR`;
- plugin remote-control order: `resolution`, `factor`, `cutoff`, `std_dev`.

This slice should make future order-changing work noisy and reviewable, without changing behavior.

### Non-goals

- Do not change generated normal-distribution traversal order.
- Do not change shared GUI settings-panel order.
- Do not change plugin remote-control order.
- Do not change plugin host parameter IDs, host parameter construction, runtime synchronization, config schemas, labels,
  ranges, defaults, units, steps, or logarithmic flags.
- Do not replace normal-distribution manual lists with generated traversal.
- Do not add a visitor macro/helper.
- Do not change `TaskExecutor` host-origin copy-back or `LiveConfig` GUI-origin setter logic.

### Durable context to read first

- `docs/audits/parameter-flow/fresh-context-handover.md`.
- `docs/audits/parameter-flow/handoff.md`, especially the visitor consumer homogeneity back-handoff.
- `docs/audits/parameter-flow/audit.md`, especially `Visitor Consumer Homogeneity Audit`.
- `docs/audits/parameter-flow/repo-map.md`, especially `Visitor Consumers And Manual Lists`.
- `docs/parameter-flow-audit.md`, especially the normal distribution typed parameter inventory and plugin remote-control
  trace.
- `rust/AGENTS.md`.
- `docs/development.md`.

### Likely files / areas

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/midi-bpm-detector-plugin/src/lib.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/handoff.md`

### Relevant boundaries / integration points

- `NormalDistributionParameters::visit` is generated from `NormalDistributionConfig` field declaration order and is used
  by inventory tests.
- The shared GUI settings panel has a user-visible manual order that differs from generated traversal.
- Plugin CLAP remote controls are host-visible and have a separate manual order that differs from both generated
  traversal and GUI settings order.
- Plugin host parameter construction and runtime copy-back are separate mappings; they should stay explicit unless a
  future synchronization slice targets them directly.

### Expected behavioral change

None. This is an order-policy guardrail slice.

### Expected structural change

- Add focused tests and/or documentation comments that pin the generated, GUI, and plugin remote-control
  normal-distribution orders as intentional current behavior.
- If adding tests, prefer narrow tests that exercise existing code paths without introducing UI or host behavior changes.
- Update durable audit docs with the policy and the next recommended implementation slice.

### Acceptance criteria

- Generated normal-distribution traversal order is explicitly guarded as `std_dev`, `factor`, `cutoff`, `resolution`.
- Shared GUI normal-distribution settings order is explicitly guarded or documented as `STD_DEV`, `RESOLUTION`, `CUTOFF`,
  `FACTOR`.
- Plugin remote-control normal-distribution order is explicitly guarded or documented as `resolution`, `factor`,
  `cutoff`, `std_dev`.
- Docs explain that these order differences are current behavior, not accidental omissions.
- No runtime behavior, plugin host IDs, config schemas, labels, ranges, defaults, units, steps, or logarithmic flags are
  changed.

### Tests / checks

From `rust/`, if code/tests change:

```sh
cargo +nightly fmt --all -- --check
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p gui
cargo test -p midi-bpm-detector-plugin
```

From repo root:

```sh
git diff --check
```

### Risks / open questions

- Adding a GUI order test may require a small test-only seam around the settings-panel parameter sequence; keep that
  seam narrow and do not turn it into a runtime abstraction unless it immediately pays for itself.
- Plugin remote-control order is host-visible. Do not "normalize" it to generated order without explicit product
  approval.
- This slice should not answer whether the three orders should eventually converge. It should only preserve current
  behavior and make the future decision explicit.

### Back-handoff requirements

Record:

- files changed;
- exact generated, GUI, and plugin remote-control normal-distribution orders after the slice;
- tests/checks run and results;
- whether any order was changed;
- any test seam or documentation decision made;
- recommended next slice.

## Prompt For Fresh Bounded Implementer

```text
[$bounded-implementer] Use the bounded implementer flow for one repository slice.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

Execute only the slice named "Normal Distribution Ordering Policy" from
docs/audits/parameter-flow/handoff.md.

Make the normal-distribution ordering policy explicit before any generated traversal/helper adoption. Preserve current
behavior: generated traversal order is std_dev, factor, cutoff, resolution; shared GUI settings order is STD_DEV,
RESOLUTION, CUTOFF, FACTOR; plugin remote-control order is resolution, factor, cutoff, std_dev. Add focused tests and/or
docs that make those differences intentional and reviewable. Do not change runtime behavior, plugin host IDs, config
schemas, labels, ranges, defaults, units, steps, logarithmic flags, TaskExecutor copy-back, LiveConfig setters, plugin
parameter construction, or remote-control ordering. Update docs/audits/parameter-flow/handoff.md with a back-handoff and
next recommended slice.
```

## Back-Handoff: Normal Distribution Ordering Policy

### Status

Complete.

### Branch / commit

- Branch: `codex/parameter-flow-audit`
- Commit: not created in this implementation turn

### Files changed

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/midi-bpm-detector-plugin/src/lib.rs`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/fresh-context-handover.md`
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Made the normal-distribution order split explicit and reviewable before any generated traversal/helper adoption. The
slice pins the three current orders:

- generated traversal: `std_dev`, `factor`, `cutoff`, `resolution`;
- shared GUI settings: `STD_DEV`, `RESOLUTION`, `CUTOFF`, `FACTOR`;
- plugin CLAP remote controls: `resolution`, `factor`, `cutoff`, `std_dev`.

### Behavioral changes

None intended. No runtime order, plugin host ID, config schema, label, range, default, unit, step, logarithmic flag,
`TaskExecutor` copy-back path, `LiveConfig` setter, plugin parameter construction, or remote-control order was changed.

### Structural changes

- Added a core inventory test that records generated normal-distribution traversal by field name, not only by label.
- Added a GUI settings comment and a narrow test-only constant documenting the intentionally manual settings order.
- Added a plugin CLAP remote-controls test that exercises `ClapPlugin::remote_controls` and asserts the
  normal-distribution page order through the actual remote-control callback path.
- Updated audit docs to state that the three orders are deliberate current behavior, not accidental omissions.

### Affected boundaries / integration points

- `NormalDistributionParameters::visit` remains generated from `NormalDistributionConfig` field order.
- Shared egui settings remain user-visible and manually ordered.
- Plugin CLAP remote controls remain host-visible and manually ordered.
- Plugin host parameter construction, `TaskExecutor` host-origin copy-back, and `LiveConfig` GUI-origin setters remain
  explicit runtime/host synchronization surfaces.

### Tests / checks

- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 7 tests.
- `cargo test -p gui normal_distribution_settings_order_is_intentional`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin normal_distribution_remote_controls_preserve_host_visible_order`: passed, 1
  test.
- `cargo +nightly fmt --all -- --check`: passed after applying rustfmt to the new plugin test.
- `cargo test -p gui`: passed, 5 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 22 tests.
- `git diff --check`: passed from repo root.
- Mutation check: temporarily swapped plugin normal-distribution remote-control `resolution` and `factor`; the focused
  plugin test failed with the expected order mismatch, then passed again after restoring the order.

### Decisions made

- Used a field-name visitor in the core inventory tests so generated order is pinned independently from labels.
- Documented the GUI settings order next to the manual list instead of replacing it with generated traversal or adding a
  runtime visitor helper.
- Tested plugin remote-control order at the CLAP trait boundary rather than only testing `MidiBpmDetectorParams`
  construction, because the order is host-visible at that boundary.

### Deviations from brief

None.

### Remaining risks

- The GUI order guard is intentionally lightweight: it documents the manual list and pins the named policy order without
  introducing a runtime abstraction around egui rendering.
- The tests do not decide whether the three orders should converge. Future work still needs explicit product/host-order
  approval before changing any user-visible or host-visible order.

### Recommended next slice

Revisit plugin host mapping surfaces now that order-sensitive normal-distribution lists are pinned. Keep the next slice
bounded to an audit or helper decision for repeated plugin adapter/host mapping code, and preserve host parameter IDs,
remote-control order, `TaskExecutor` copy-back, and `LiveConfig` setter behavior unless explicitly scoped otherwise.

## Coordinator Correction: Normal Distribution Order Alignment

### Status

Complete and verified, pending commit.

This supersedes the immediately preceding `Normal Distribution Ordering Policy` back-handoff. The previous slice treated
the generated order, GUI settings order, and plugin CLAP remote-control order as three deliberate orders. The product
decision is now that the shared egui settings order is canonical.

### Branch / commit

- Branch: `codex/parameter-flow-audit`
- Commit: not created in this coordinator turn yet.

### Files changed

- `rust/crates/bpm_detection_core/src/parameters.rs`
- `rust/crates/gui/src/add_slider.rs`
- `rust/crates/gui/src/config_ui.rs`
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`
- `rust/crates/midi-bpm-detector-plugin/src/lib.rs`
- `rust/crates/midi-bpm-detector-plugin/src/task_executor.rs`
- `rust/crates/midi-bpm-detector-plugin/src/bpm_detector_configuration.rs`
- `docs/audits/parameter-flow/audit.md`
- `docs/audits/parameter-flow/fresh-context-handover.md`
- `docs/audits/parameter-flow/repo-map.md`
- `docs/audits/parameter-flow/handoff.md`

### Summary

Aligned normal-distribution ordering to the canonical GUI order:

- generated traversal: `std_dev`, `resolution`, `cutoff`, `factor`;
- shared GUI settings: generated traversal via `NormalDistributionParameters::visit(...)`;
- plugin parameter construction: `std_dev`, `resolution`, `cutoff`, `factor`;
- plugin host-origin copy-back: `std_dev`, `resolution`, `cutoff`, `factor`;
- `LiveConfig` normal-distribution accessor implementation order: `std_dev`, `resolution`, `cutoff`, `factor`;
- plugin CLAP remote controls: `std_dev`, `resolution`, `cutoff`, `factor`.

### Behavioral changes

Plugin CLAP remote-control order changed from `resolution`, `factor`, `cutoff`, `std_dev` to
`std_dev`, `resolution`, `cutoff`, `factor`. Parameter IDs, labels, ranges, defaults, units, steps, logarithmic flags,
config schemas, and runtime synchronization semantics are unchanged.

### Structural changes

- Reordered `NormalDistributionConfig` fields so the proc macro generates the canonical traversal order naturally.
- Added `SlideAdder`'s homogeneous `NormalDistributionParameterVisitor` impl through the existing generic
  `parameter(...)` fallback.
- Replaced the manual normal-distribution settings-panel list with generated traversal.
- Reordered plugin normal-distribution host parameter declarations and construction to match the canonical order.
- Reordered host-origin copy-back and `LiveConfig` accessor methods for source-level consistency.
- Updated the CLAP remote-controls test to guard the canonical order at the host-visible boundary.
- Updated durable audit docs to record that the previous host-visible order was intentionally superseded.

### Tests / checks

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 7 tests.
- `cargo test -p gui`: passed, 5 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 22 tests.
- `git diff --check`: passed from repo root.
- `cargo clippy -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.

### Remaining risks

- The CLAP remote-control order is host-visible. This change is intentional because the GUI settings order is now the
  canonical product order.
- Plugin host mapping and runtime synchronization remain explicit and repetitive. Do not hide those behind a helper until
  the next slice audits IDs, callbacks, host handles, copy-back timing, and GUI refresh side effects together.

### Recommended next slice

Revisit plugin host mapping surfaces now that normal-distribution ordering is aligned. Keep the next slice bounded to an
audit/helper decision for repeated plugin adapter and host mapping code. Preserve host parameter IDs, the canonical
remote-control order, `TaskExecutor` copy-back semantics, and `LiveConfig` setter semantics unless explicitly scoped
otherwise.
