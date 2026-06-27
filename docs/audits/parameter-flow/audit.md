# Parameter Flow Audit

This is the durable coordinator log for the parameter mapping/refactor audit. The detailed inventory and flow trace from
the initial audit live in `docs/parameter-flow-audit.md`; this file records decisions, current direction, and bounded
implementation slicing.

## Durable State Versus Chat State

Durable repo state:

- `docs/parameter-flow-audit.md` contains the current detailed inventory, flow trace, and invariant audit.
- `docs/parameter-audit-handoff.md` is an older top-level restart note.
- `docs/audits/parameter-flow/` is now the canonical coordinator workspace required by the repo workflow.

Current branch checkpoint:

- Branch: `codex/parameter-flow-audit`.
- The dynamic macro prototype, dynamic metadata-spec split, normal-distribution migration, and GUI migration have been
  implemented.
- Audit docs now include bounded implementer back-handoffs for those slices.

Assumptions from current coordination:

- The user wants a macro/design direction for the mechanical parameter-group pattern.
- The user explicitly agrees that macro-generated code can be justified here, but rejects macro APIs that replace normal
  Rust structs with a large declarative invocation DSL.
- The next implementation/design slice should stay bounded and should not try to fix all parameter sync semantics at once.

## Findings From Completed Audit Passes

Completed:

1. Parameter declaration inventory.
2. Flow trace from defaults/config through egui, plugin host params, remote controls, runtime tasks, worker snapshots, and
   atomics.
3. Invariant and failure-mode audit.

Key findings:

- There are 20 typed `Parameter` declarations:
  - 2 GUI/display;
  - 3 static BPM model;
  - 4 normal distribution static submodel;
  - 11 dynamic scoring.
- Dynamic scoring has the strongest shared traversal, but visitor methods currently have default implementations, so
  missing visitor behavior can still compile.
- Static BPM, normal distribution, GUI/display, and output/runtime state are more manually wired.
- GUI/display interpolation params currently ride dynamic update paths in desktop, wasm, and plugin modes.
- Plugin host-origin dynamic sync is overloaded: dynamic scoring, GUI/display, and `send_tempo` all share the dynamic
  task path.
- Plugin host-origin dynamic sync assigns `interpolation_duration` twice.
- Parameter-like atomics such as `send_tempo`, `enable_midi_clock`, and `daw_port` have bespoke persistence/sync rules.

## Macro/Homogeneity Evaluation

The current dynamic-config machinery repeats the same field list across:

- `DynamicBPMDetectionConfig` fields;
- `DynamicBPMDetectionConfigAccessor` getter/setter methods;
- the accessor impl for `()`;
- the accessor impl for `DynamicBPMDetectionConfig`;
- `DynamicBPMDetectionParameters` constants;
- `DynamicBPMDetectionParameters::visit`;
- `DynamicBPMDetectionParameterVisitor` methods;
- tests that re-list labels or persistent keys.

This pattern gives useful readability today because the repetition makes omissions look visually wrong, but it scales
poorly. It also explains why static BPM, normal distribution, and GUI config have not been made fully homogeneous: paying
the same boilerplate tax for every group would make the code awkward.

Coordinator judgment before the first proof attempt:

- A macro-generated parameter group is justified here.
- The macro should make the field list the source of truth for the mechanical pieces.
- The macro should preserve existing public type names and behavior for the first slice.
- The macro should not own runtime update policy. Static/dynamic/gui/output synchronization must remain explicit in
  desktop, wasm, and plugin code.
- Start with dynamic config only, because it already has the fullest pattern and the strongest test coverage.
- Prefer a local `macro_rules!` macro first. Do not introduce a proc-macro crate unless a bounded implementation attempt
  shows that declarative syntax becomes unreadable or cannot preserve the current API cleanly.

## Rejected Macro Proof

The first implementation attempt used a dynamic-specific `macro_rules!` macro:

```rust
macro_rules! dynamic_bpm_detection_parameter_group { ... }

dynamic_bpm_detection_parameter_group! {
    beats_lookback: u8 => BEATS_LOOKBACK, set_beats_lookback {
        label: "Beats Lookback",
        unit: None,
        range: 2.0..=32.0,
        step: 1.0,
        logarithmic: false,
        default: 8,
    },
    ...
}
```

That proof was rejected and scratched from production code.

Why it was rejected:

- The macro was tied to the dynamic BPM group instead of being a general parameter-config facility.
- The macro body contained a large block of generated Rust code, which made ordinary editing and compiler errors harder
  to reason about.
- The invocation created a bespoke mini-language that was noisier than the original Rust and likely to confuse humans and
  tools.
- It did not match the desired shape: keep the config struct as normal Rust and generate mechanical companion items from
  that struct.

The production file `rust/crates/bpm_detection_core/src/parameters.rs` has been restored to the pre-proof version.

## Revised Macro Requirements

Acceptable macro direction:

- Keep the config struct visible as ordinary Rust.
- Prefer derive or attribute-macro ergonomics similar to serde:
  - field names remain real struct fields;
  - field attributes carry metadata where needed;
  - generated code appears as companion trait/impl/parameter/visitor machinery.
- The macro must be generic over parameter groups, not named after dynamic BPM.
- The macro may generate accessor traits, concrete accessor impls, parameter constants, defaults, validation, and
  traversal APIs, but must not decide runtime synchronization policy.
- The macro API should make adding a field feel like adding a Rust field plus small metadata attributes, not editing a
  custom DSL block.

Open design questions:

- Is an attribute macro on the struct more appropriate than a derive macro because it may need to generate several
  companion items, including a trait definition?
- Can `Parameter` metadata live cleanly in field attributes without making the struct unreadable?
- Should the generated traversal keep the current visitor trait, or should a later slice replace visitors with a
  generated typed enumeration/catalog?
- Should `()` accessor impls remain generated, or should default-only metadata stop needing fake config accessors?

## Candidate Macro Shapes

### Plain Rust Plus Small Helper Macros

This would keep all config structs and traits hand-written, but use small local macros for the most repetitive impl
blocks, for example implementing an accessor trait for a concrete struct.

Pros:

- Low tooling risk.
- Keeps most Rust visible.
- Does not require a proc-macro crate.

Cons:

- Still requires a separate field list for parameter constants and traversal.
- Does not solve the real source-of-truth problem.
- Easy to drift back into several small bespoke macros.

Assessment: useful as a narrow cleanup tool, but not enough for the main parameter-group problem.

### Declarative `macro_rules!` Parameter Group

This was the rejected proof. It moves the field list and metadata into a macro invocation and generates the config struct,
accessor trait, impls, constants, and visitor.

Pros:

- Can be implemented locally.
- Can remove a lot of boilerplate.

Cons:

- Replaces ordinary Rust structs with a custom mini-language.
- Produces hard-to-debug generated blocks from a large macro body.
- Becomes group-specific unless it accepts a very large and awkward parameter surface.
- Does not match the desired serde-like editing experience.

Assessment: rejected. Do not repeat this shape.

### Derive Macro On Normal Struct

Example sketch:

```rust
#[derive(Clone, Debug, Derivative, Serialize, Deserialize, ParameterGroup)]
#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DynamicBPMDetectionConfig {
    #[parameter(label = "Beats Lookback", range = 2.0..=32.0, step = 1.0, default = 8)]
    pub beats_lookback: u8,

    #[parameter(label = "Normal distribution", range = 0.0..=1.0, default = OnOff::On(1.0))]
    pub normal_distribution_weight: OnOff<f32>,
}
```

Pros:

- Keeps the struct as ordinary Rust.
- Field attributes mirror serde ergonomics.
- Can generate impls and companion items while preserving public names.

Cons:

- Derive macros usually implement traits; generating trait definitions, type aliases, visitor traits, and companion
  structs from a derive is possible but less idiomatic.
- Requires a proc-macro crate.
- Field metadata can become noisy if every attribute repeats label/range/default details.

Assessment: plausible, especially if the derive only generates impls for predeclared generic traits. Less ideal if it
must generate many named companion item definitions.

### Attribute Macro On Normal Struct

Example sketch:

```rust
#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DynamicBPMDetectionConfig {
    #[parameter(label = "Beats Lookback", range = 2.0..=32.0, step = 1.0, default = 8)]
    pub beats_lookback: u8,

    #[parameter(label = "Normal distribution", range = 0.0..=1.0, default = OnOff::On(1.0))]
    pub normal_distribution_weight: OnOff<f32>,
}
```

Pros:

- Keeps the struct as ordinary Rust.
- More honest than derive when generating multiple companion items, including trait definitions.
- Can preserve existing public names and generate setter names from field names.
- Can be generic across dynamic, static, normal, and GUI groups.

Cons:

- Requires a proc-macro crate.
- Attribute macro output must preserve the input struct and its derives carefully.
- The generated API needs careful design so rust-analyzer and errors point back to useful spans.

Assessment: current best fit. It matches the user's accepted macro style most closely: normal Rust item plus metadata
attributes, with mechanical companion code generated next to it.

## Chosen Macro API Direction

Use an attribute proc macro on ordinary Rust config structs.

The macro should preserve the input struct as the obvious source of truth and generate only the mechanical companion
items around it. This is intentionally closer to serde-style field attributes than to a custom parameter DSL.

Proposed shape for the first implementation slice:

```rust
#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DynamicBPMDetectionConfig {
    #[parameter(label = "Beats Lookback", range = 2.0..=32.0, step = 1.0, default = 8)]
    pub beats_lookback: u8,

    #[parameter(label = "Normal distribution", range = 0.0..=1.0, default = OnOff::On(1.0))]
    pub normal_distribution_weight: OnOff<f32>,
}
```

The macro should generate the same public surface currently written by hand for dynamic config:

- the accessor trait with one getter and one setter per annotated parameter field;
- `impl Accessor for ()`;
- `impl Accessor for DynamicBPMDetectionConfig`;
- the `DefaultDynamicBPMDetectionParameters` alias;
- the `DynamicBPMDetectionParameters<Config>` companion type;
- one `Parameter<Config, ValueType>` associated const per annotated field;
- `visit`;
- the visitor trait with one field-specific method per annotated field;
- `Default` and `validate` for the concrete config, if requested by the group macro.

The macro should not generate or own runtime synchronization. Desktop, wasm, plugin, egui, atomics, and host automation
flows stay explicit outside the macro.

### Attribute Surface

The group-level attribute should provide names for generated public items instead of deriving names that could surprise
readers:

```rust
#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
```

Field-level `#[parameter(...)]` attributes should stay small:

- required: `label`, `range`, `default`;
- optional: `unit`, `step`, `logarithmic`, `const_name`, `setter`;
- defaults:
  - `unit = None`;
  - `step = 0.0`;
  - `logarithmic = false`;
  - `const_name` is the field name converted to screaming snake case;
  - `setter` is `set_` plus the field name.

Labels should remain explicit for now. Inferring labels from field names would reduce typing but would also hide UX text
changes in generated behavior.

### Crate Shape

Add a dedicated proc-macro crate, tentatively `rust/crates/parameter_macros`, using `syn`, `quote`, and `proc-macro2`.

The existing `parameter` crate should remain the runtime home for `Parameter`, `Asf64`, and `OnOff`. The macro crate
should generate references to the runtime `parameter` crate but should not depend on `bpm_detection_core`, `gui`,
`desktop`, `wasm`, or `midi-bpm-detector-plugin`.

If the path to the runtime crate becomes awkward, support an optional group-level override:

```rust
#[parameter_group(parameter_crate = parameter, ...)]
```

### Static Config Caveat

Dynamic config, normal distribution config, and GUI config are plain parameter groups. Static BPM config is almost a
plain group but its current accessor trait also carries computed methods such as `index_to_bpm`, `highest_bpm`, and
`lowest_bpm`.

Do not force that special case into the first macro implementation. The likely follow-up design is to generate a
field-accessor trait for parameter fields and keep a small hand-written extension trait for static computed methods.
That lets the common parameter machinery become homogeneous without turning the macro into another bespoke DSL.

### Why Not `macro_rules!`

`macro_rules!` can still help with tiny local repetitions, but it is not the right source-of-truth mechanism here.
The accepted source should be a real struct item with real Rust fields, serde/derivative attributes, and small
parameter metadata attributes. A `macro_rules!` invocation would either recreate the rejected custom field list or
require enough nested syntax to become harder to edit than the current boilerplate.

### Why Attribute Over Derive

A derive macro is plausible, but the generated surface is larger than an ordinary derive: it includes trait definitions,
companion structs, type aliases, parameter constants, traversal, defaulting, and validation. An attribute macro is more
honest about rewriting one item into that item plus generated siblings.

The important ergonomic property is not whether the spelling says `derive`; it is that the source remains a normal Rust
struct and that errors point back to field attributes.

## Current Decision

The first implementation slice prototyped the chosen attribute proc-macro API on dynamic config only.

Keep all generated public names and behavior equivalent to the previous hand-written dynamic config API. Do not apply the
macro to static BPM, normal distribution, GUI config, or plugin/egui mapping code until a later reviewed slice.

## Coordinator Review: Attribute Proc-Macro Prototype

Review checkpoint: `codex/parameter-flow-audit` at `a5fc659 Improve parameter macro diagnostics`, two commits ahead of
`upstream/main`.

Completed slice commits:

- `431d0d3 Add dynamic parameter group macro prototype`
- `a5fc659 Improve parameter macro diagnostics`

The prototype matches the accepted direction:

- `DynamicBPMDetectionConfig` remains an ordinary Rust struct with real fields.
- Dynamic field metadata lives in small `#[parameter(...)]` attributes.
- The macro generates the dynamic accessor trait, concrete accessor impl, default metadata catalog, parameter constants,
  visitor trait, traversal, default impl, and validation.
- The rejected dynamic-specific `macro_rules!` DSL is absent from production code.
- Static BPM, normal distribution, GUI config, and runtime synchronization policy were not migrated in this slice.
- The diagnostics follow-up improved missing/unknown/duplicate macro-argument errors and added fixture-based diagnostic
  tests without adding a trybuild dependency.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, including 4 diagnostics tests and 2 parameter-group tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- `git diff --check`: passed.

Coordinator judgment: the dynamic attribute macro is solid enough to keep, but the generated default catalog still exposes
the old `Parameters<()>` compatibility bridge. That bridge is public and can panic if code calls `Parameter::get` or
`Parameter::set` on metadata-only constants. Before applying the macro to more groups, split metadata-only parameter specs
from config-bound parameters for the generated dynamic catalog.

## Coordinator Review: Metadata-Only Dynamic Parameter Specs

Review checkpoint: `codex/parameter-flow-audit`, in the coordinator checkpoint that includes the metadata-spec
implementation on top of `a5fc659 Improve parameter macro diagnostics`.

The slice matches the brief:

- `parameter::ParameterSpec<ValueType>` now represents metadata-only parameter declarations.
- The generated `DefaultDynamicBPMDetectionParameters` catalog is a concrete spec catalog, not a
  `DynamicBPMDetectionParameters<()>` alias.
- The generated macro output no longer includes `impl DynamicBPMDetectionConfigAccessor for ()`.
- `DynamicBPMDetectionParameters<Config>` remains the config-bound `Parameter<Config, T>` catalog.
- Plugin dynamic remote-control ordering and dynamic host reads now use
  `DynamicBPMDetectionParameters<DynamicBPMDetectionConfig>::visit`.
- Static BPM, normal distribution, and GUI fake-config aliases remain deliberately unchanged.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter`: passed, 0 tests.
- `cargo test -p parameter_macros`: passed, including 4 diagnostics tests and 2 parameter-group tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 1 test.
- `cargo test -p bpm_detection_core`: passed, 3 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- `git diff --check`: passed.

Coordinator judgment: the spec split is an improvement and removes the generated dynamic fake-config panic surface. The
remaining `Parameters<()>` aliases are now hand-written debt in static BPM, normal distribution, and GUI config. The next
bounded migration should apply the attribute macro to `NormalDistributionConfig`, because it is a small plain core group
without static BPM's computed methods and without adding a new macro dependency to the GUI crate.

## Coordinator Review: NormalDistributionConfig Macro Migration

Review checkpoint: `codex/parameter-flow-audit`, in the coordinator checkpoint that includes the normal-distribution
migration on top of the metadata-spec slice.

The slice matches the brief:

- `NormalDistributionConfig` remains an ordinary Rust struct with real fields and the existing serde/derivative behavior.
- Normal distribution metadata now lives in field-level `#[parameter(...)]` attributes.
- `DefaultNormalDistributionParameters` is generated as a `ParameterSpec<T>` metadata catalog.
- `NormalDistributionParameters<Config>` remains the config-bound parameter catalog.
- The generated macro output removes the normal distribution fake `impl NormalDistributionConfigAccessor for ()`.
- Plugin normal distribution host params remain manually enumerated under the static parameter group.
- Static BPM, dynamic config, and GUI config were not migrated in this slice.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p bpm_detection_core parameter_inventory_tests`: passed, 2 tests.
- `cargo test -p bpm_detection_core`: passed, 4 tests.
- `cargo test -p gui`: passed, 1 test.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo clippy -p parameter_macros -p bpm_detection_core -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`:
  passed.
- `git diff --check`: passed.

Coordinator judgment: the normal distribution migration is accepted. The next bounded migration should apply the same
macro pattern to `GUIConfig`, while keeping GUI/display runtime update semantics unchanged. Static BPM remains later
because its accessor trait still carries computed methods.

## Coordinator Review: GUIConfig Macro Migration

Review checkpoint: `codex/parameter-flow-audit`, in the coordinator checkpoint that includes the GUI migration on top of
the normal-distribution slice.

The slice matches the brief:

- `GUIConfig` remains an ordinary Rust struct with real public fields, serde derives, and `deny_unknown_fields`.
- GUI metadata now lives in field-level `#[parameter(...)]` attributes.
- `DefaultGUIParameters` is generated as a `ParameterSpec<T>` metadata catalog.
- `GUIParameters<Config>` remains the config-bound parameter catalog.
- The generated macro output removes the GUI fake `impl GUIConfigAccessor for ()`.
- Desktop, wasm, and plugin runtime wrappers continue to implement `GUIConfigAccessor`.
- Plugin GUI/display host parameter IDs remain `interpolation_duration` and `interpolation_curve`.
- Static BPM was not migrated in this slice.

Fresh coordinator verification:

- `cargo +nightly fmt --all -- --check`: passed.
- `cargo test -p parameter_macros`: passed, 6 tests.
- `cargo test -p gui`: passed, 2 tests.
- `cargo test -p midi-bpm-detector-plugin`: passed, 21 tests.
- `cargo test -p desktop`: passed, 13 tests.
- `cargo test -p wasm --target wasm32-unknown-unknown`: passed, 1 test.
- `cargo clippy -p parameter_macros -p gui -p midi-bpm-detector-plugin --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

Coordinator judgment: the GUI migration is accepted. All plain typed parameter groups now use the attribute macro. Static
BPM remains intentionally hand-written because its accessor trait mixes field accessors with computed methods
(`index_to_bpm`, `highest_bpm`, and `lowest_bpm`). The next bounded slice should split those computed methods from the
field accessor contract before applying the macro to `StaticBPMDetectionConfig`.

## Non-Goals For The Completed Macro Implementation Slice

- Do not change host/GUI sync policy.
- Do not fix GUI/display params riding the dynamic update path.
- Do not change config TOML schema or shipped default values.
- Do not implement a group-specific `macro_rules!` replacement like the rejected proof.
- Do not migrate static BPM, normal distribution, or GUI config yet.
- Do not design static computed methods into the first macro API.
- Do not introduce egui, nih-plug, desktop, wasm, or plugin dependencies into the macro/runtime parameter crates.

## Recommended Slice Sequence

1. Implement the attribute proc-macro prototype and apply it only to `DynamicBPMDetectionConfig`.
2. Coordinator review of the generated API, diagnostics, rust-analyzer ergonomics, and diff readability.
3. Split generated dynamic default catalogs away from the `Parameters<()>` fake-config bridge.
4. Apply the attribute macro to `NormalDistributionConfig`.
5. Apply the attribute macro to `GUIConfig`.
6. Split static BPM computed methods from the static parameter field accessor contract.
7. Apply the attribute macro to `StaticBPMDetectionConfig`.
8. Revisit egui/plugin host mapping surfaces once all typed groups are homogeneous.
9. Separately address runtime semantics: GUI/display update path, plugin dynamic task overload, duplicate interpolation
   assignment, and parameter-like atomics.
