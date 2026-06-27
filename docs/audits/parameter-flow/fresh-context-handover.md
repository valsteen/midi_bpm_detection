# Parameter Flow Fresh Context Handover

Use this file to restart the parameter-flow audit from a fresh Codex context.

## Current Branch And Working Tree

Branch: `codex/parameter-flow-audit`, tracking `upstream/codex/parameter-flow-audit`.

The branch now contains the completed parameter macro slices:

- `docs/audits/parameter-flow/`
- `docs/parameter-audit-handoff.md`
- `docs/parameter-flow-audit.md`
- `rust/crates/parameter_macros/`
- dynamic and normal-distribution parameter-group macro wiring in `rust/crates/bpm_detection_core/src/parameters.rs`
- GUI parameter-group macro wiring in `rust/crates/gui/src/config.rs`
- static BPM computed-method split in `rust/crates/bpm_detection_core/src/parameters.rs`

Reviewed implementation commits on this branch:

- `431d0d3 Add dynamic parameter group macro prototype`
- `a5fc659 Improve parameter macro diagnostics`
- `d3dfead Split parameter specs and migrate normal distribution`
- `83bae5a Migrate GUI parameters to parameter group macro`
- `3078c83 Split static BPM computed methods`
- `bdab497 Stabilize parameter macro diagnostic tests`

Draft PR: <https://github.com/valsteen/midi_bpm_detection/pull/20>.

## What The Audit Is About

The project has many parameter declarations and mappings:

- typed `Parameter` declarations in core/gui code;
- loaded config and defaults;
- plugin load-time host parameters;
- egui widgets;
- conversions from/to GUI state;
- conversions from/to plugin runtime config;
- dynamic runtime updates;
- parameter-like atomics such as tempo-output toggles and ports.

The audit found that these flows are currently correct-ish but mechanically spread out. The highest-value near-term work
is to reduce "forgot to map this field" risk while preserving strong typing and explicit runtime ownership.

## Important Current Decision

Do not repeat the rejected `macro_rules!` shape.

Rejected shape:

- a dynamic-specific macro such as `dynamic_bpm_detection_parameter_group!`;
- a custom field-list DSL that replaces the normal Rust struct;
- a large hidden block of generated Rust inside a declarative macro.

Accepted direction:

- keep config structs as ordinary Rust;
- attach small serde-like metadata attributes to fields;
- use a generic attribute proc macro to generate mechanical companion items;
- keep runtime synchronization policy explicit outside the macro.

Preferred sketch:

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

## Completed Slice

The completed slice is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Attribute Parameter Group Macro Prototype`

Completed scope:

- implement a generic attribute proc-macro prototype;
- apply it only to `DynamicBPMDetectionConfig`;
- preserve all current dynamic public names and behavior;
- do not migrate static BPM, normal distribution, or GUI config yet.

Key public items that must remain available:

- `DynamicBPMDetectionConfigAccessor`
- `DefaultDynamicBPMDetectionParameters`
- `DynamicBPMDetectionParameters<Config>`
- `DynamicBPMDetectionParameterVisitor<Config>`

Dynamic visitor order must remain:

1. `beats_lookback`
2. `normal_distribution_weight`
3. `time_distance_weight`
4. `velocity_current_note_weight`
5. `velocity_note_from_weight`
6. `in_beat_range_weight`
7. `multiplier_weight`
8. `subdivision_weight`
9. `octave_distance_weight`
10. `pitch_distance_weight`
11. `high_tempo_bias_weight`

## Completed Follow-Up Slice

The completed follow-up slice is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Metadata-Only Dynamic Parameter Specs`

Completed scope:

- add `parameter::ParameterSpec<ValueType>`;
- make `DefaultDynamicBPMDetectionParameters` expose specs instead of `Parameter<(), T>`;
- remove the generated `DynamicBPMDetectionConfigAccessor for ()` bridge;
- preserve `DynamicBPMDetectionParameters<Config>` as the config-bound parameter catalog;
- leave static BPM, normal distribution, and GUI config hand-written.

## Completed Normal Distribution Slice

The completed normal distribution slice is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Attribute Macro For NormalDistributionConfig`

Completed scope:

- apply the generic `#[parameter_group(...)]` macro to `NormalDistributionConfig`;
- make `DefaultNormalDistributionParameters` expose specs instead of `Parameter<(), T>`;
- remove the hand-written `NormalDistributionConfigAccessor for ()` bridge;
- preserve `NormalDistributionParameters<Config>` as the config-bound parameter catalog;
- leave GUI config and static BPM config hand-written.

## Completed GUI Slice

The completed GUI slice is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Attribute Macro For GUIConfig`

Completed scope:

- apply the generic `#[parameter_group(...)]` macro to `GUIConfig`;
- make `DefaultGUIParameters` expose specs instead of `Parameter<(), T>`;
- remove the hand-written `GUIConfigAccessor for ()` bridge;
- preserve `GUIParameters<Config>` as the config-bound parameter catalog;
- leave static BPM config hand-written.

## Completed Static Computed-Method Split

The completed static split is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Static BPM Computed-Method Split`

Completed scope:

- split `StaticBPMDetectionConfigAccessor` so it contains only `bpm_center`, `bpm_range`, `sample_rate`, and their
  setters;
- add `StaticBPMDetectionComputed` as the explicit computed-method extension for `index_to_bpm`, `highest_bpm`, and
  `lowest_bpm`;
- remove the static fake `StaticBPMDetectionConfigAccessor for ()` bridge;
- make `DefaultStaticBPMDetectionParameters` expose `ParameterSpec<T>` metadata specs;
- leave `StaticBPMDetectionConfig` hand-written and not annotated with `#[parameter_group(...)]`.

## Next Slice To Execute

The active next slice is documented in:

- `docs/audits/parameter-flow/handoff.md`

Slice name:

- `Attribute Macro For StaticBPMDetectionConfig`

Scope:

- apply the existing generic `#[parameter_group(...)]` macro to `StaticBPMDetectionConfig`;
- include only the static BPM parameter fields: `bpm_center`, `bpm_range`, and `sample_rate`;
- keep `normal_distribution` as a nested config field outside the static parameter field group;
- preserve `StaticBPMDetectionComputed`, inherent computed methods, static labels/ranges/defaults, serde fields, plugin
  host parameter IDs, GUI histogram behavior, and runtime update routing.

## What To Read First In A Fresh Chat

Read these files, in this order:

1. `docs/audits/parameter-flow/fresh-context-handover.md`
2. `docs/audits/parameter-flow/handoff.md`
3. `docs/audits/parameter-flow/audit.md`
4. `docs/audits/parameter-flow/repo-map.md`
5. `docs/parameter-flow-audit.md`
6. `rust/AGENTS.md`
7. `docs/development.md`

The full audit docs contain extra context; this file is the compact restart path.

## Avoid These Detours

- Do not re-litigate the dynamic-specific `macro_rules!` macro unless comparing against the accepted attribute macro
  direction.
- Do not start with GUI/egui streamlining. That becomes cleaner after the parameter groups are homogeneous.
- Do not fold plugin runtime sync, egui widgets, output atomics, or host automation into the macro slice.
- Do not solve static BPM computed methods in the first macro slice. Static config has extra computed methods such as
  `index_to_bpm`, `highest_bpm`, and `lowest_bpm`; handle that after the dynamic prototype proves the macro shape.

## Useful Commands

From repo root:

```sh
git status --short --branch
```

From `rust/`, after the macro prototype:

```sh
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p bpm_detection_core
```

## Prompt For Fresh Audit Coordinator Chat

```text
[$repo-audit-coordinator] Use the repo audit coordinator flow.

Read first:
- docs/audits/parameter-flow/fresh-context-handover.md
- docs/audits/parameter-flow/handoff.md
- docs/audits/parameter-flow/audit.md
- docs/audits/parameter-flow/repo-map.md
- docs/parameter-flow-audit.md
- rust/AGENTS.md
- docs/development.md

We are continuing the parameter-flow audit from a fresh context. The dynamic attribute macro prototype, metadata-spec
split, NormalDistributionConfig migration, GUIConfig migration, and Static BPM computed-method split are implemented.
StaticBPMDetectionConfig is the only typed parameter group left outside the macro path. Do not revisit the rejected
dynamic-specific macro_rules DSL except as historical context. First confirm the current branch and working tree, then
either prepare the bounded implementer prompt for the "Attribute Macro For StaticBPMDetectionConfig" slice or continue
coordinator review if the docs have drifted.
```

## Prompt For Fresh Implementer Chat

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
parameter fields (`bpm_center`, `bpm_range`, `sample_rate`) in the generated parameter group, keep
`normal_distribution` nested but outside the generated static field group, and preserve StaticBPMDetectionComputed,
inherent computed methods, labels, ranges, defaults, serde fields, plugin host parameter IDs, GUI histogram behavior, and
desktop/wasm/plugin runtime update routing. Update docs/audits/parameter-flow/handoff.md with a back-handoff.
```
