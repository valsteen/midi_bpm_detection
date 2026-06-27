# Parameter Flow Repo Map

This map covers the parameter mapping/refactor audit. It focuses on Rust parameter declarations, config loading,
shared egui controls, plugin host parameters, runtime update paths, and tests. It does not cover the Bitwig extension
except where plugin host parameters cross into the tempo bridge.

## Current Branch And Working Tree

- Branch observed during coordination: `codex/parameter-flow-audit`, tracking `upstream/codex/parameter-flow-audit`.
- The branch now includes completed macro slices for dynamic config, dynamic metadata specs, normal distribution, GUI,
  and static BPM, plus the static BPM computed-method split.
- Audit docs live in:
  - `docs/parameter-flow-audit.md`
  - `docs/parameter-audit-handoff.md`
  - `docs/audits/parameter-flow/*`

## Relevant Crates

- `rust/crates/parameter`
  - Defines `Parameter<Config, ValueType>`, `Asf64`, and `OnOff<T>`.
  - `Parameter` stores label, unit, range, step, logarithmic flag, default, getter, and setter.
  - `OnOff<T>` serializes a logical value as `{ enabled, value }`.
  - `ParameterSpec<ValueType>` is the metadata-only shape used by generated default catalogs.

  - `rust/crates/parameter_macros`
  - Defines the generic `#[parameter_group(...)]` proc macro.
  - Generates the dynamic config accessor trait, `Parameter` constants, default impl, validation, visitor trait, and
    traversal from ordinary struct fields plus small `#[parameter(...)]` metadata.
  - Generated default catalogs use `ParameterSpec<T>` and do not need fake `Config = ()` accessors.
  - Unannotated fields in a parameter group are treated as nested config fields: generated `Default` initializes them
    with `Default::default()`, generated `validate()` calls `self.<field>.validate()?`, and generated parameter traversal
    omits them.
  - Must stay free of egui, nih-plug, desktop, wasm, and plugin dependencies.

- `rust/crates/bpm_detection_core`
  - Owns algorithm config shapes:
    - `StaticBPMDetectionConfig`
    - `DynamicBPMDetectionConfig`
    - `NormalDistributionConfig`
  - Dynamic config, normal distribution, and static BPM now use the generated parameter-group pattern:
    - config struct;
    - accessor trait;
    - accessor impl for the concrete config;
    - metadata spec constants;
    - config-bound parameter constants;
    - visitor trait;
    - validation through generated traversal.
  - Static BPM computed methods remain explicit outside the generated parameter group through
    `StaticBPMDetectionComputed`.

- `rust/crates/gui`
  - Owns `GUIConfig`, `GUIConfigAccessor`, `GUIParameters`, and reusable egui parameter controls.
  - GUI config now uses the generated parameter-group pattern:
    - config struct;
    - accessor trait;
    - accessor impl for the concrete config;
    - metadata spec constants;
    - config-bound parameter constants;
    - visitor trait;
    - validation through generated traversal.
  - `BPMDetectionGUI::settings_panel` uses generated traversal for GUI/display, static BPM, normal distribution, and
    dynamic scoring params.
  - `SlideAdder` renders typed `Parameter` values and implements:
    - `GUIParameterVisitor` through the generic `parameter(...)` fallback;
    - `NormalDistributionParameterVisitor` through the generic `parameter(...)` fallback;
    - `StaticBPMDetectionParameterVisitor` through the generic `parameter(...)` fallback;
    - `DynamicBPMDetectionParameterVisitor` through explicit field methods because dynamic has both plain and `OnOff`
      rendering paths.

- `rust/crates/desktop`
  - Loads `DesktopConfig` from built-in TOML plus optional user config.
  - `DesktopBaseConfig` implements static, normal, dynamic, GUI, and output accessors for the shared egui app.
  - Static/normal setters propagate static config to the controller.
  - Dynamic setters propagate dynamic config to the controller.
  - GUI/display setters currently propagate dynamic config even though interpolation fields are display-only.
  - `midi.send_tempo` and `midi.enable_midi_clock` are atomic runtime state, not typed `Parameter` values.

- `rust/crates/wasm`
  - Loads `WASMConfig` from built-in TOML.
  - `BaseConfig` implements the shared accessors for egui.
  - Static and dynamic updates are sent through browser queue items and delayed before recompute.
  - GUI/display setters currently send dynamic queue items.

- `rust/crates/midi-bpm-detector-plugin`
  - Loads `PluginConfig` from built-in TOML.
  - `MidiBpmDetectorParams` maps config values into `nih-plug` host parameters.
  - Host-origin updates are coalesced on sample time, then copied from host params into shared `PluginConfig`.
  - GUI-origin updates write through host params with `ParamSetter`, then delayed tasks update the BPM model from shared
    config.
  - `send_tempo` and `daw_port` are host-visible but not typed `Parameter` values.

- `rust/crates/sync`
  - Owns serializable atomic wrappers used by parameter-like runtime state:
    - `ArcAtomicBool`
    - `ArcAtomicOptionNonZeroU16`
    - `ArcAtomicOptionUsize`
    - `ArcAtomicOptionU64`

## Relevant Config Files

- `rust/crates/desktop/config/base_config.toml`
- `rust/crates/midi-bpm-detector-plugin/config/base_config.toml`
- `rust/crates/wasm/config/base_config.toml`

These shipped defaults are not simple mirrors of code defaults. Implementation slices must preserve both code-level
defaults and shipped TOML behavior unless explicitly scoped otherwise.

## Relevant Docs

- `docs/parameter-flow-audit.md`
  - Current detailed inventory, flow trace, invariant audit, and findings.
- `docs/audits/parameter-flow/audit.md`
  - Coordinator decision log and current macro direction.
- `docs/audits/parameter-flow/handoff.md`
  - Current slice brief and prompt for a fresh bounded implementer.
- `docs/architecture.md`
  - Stable architecture narrative, including plugin parameter synchronization.
- `docs/runtime-lifecycle.md`
  - Detailed host-origin and GUI-origin plugin flows.
- `docs/plugin-flow.md`
  - Plugin realtime callback and deferred parameter update flow.
- `docs/native-midi-flow.md`
  - Desktop/native MIDI config propagation flow.
- `docs/bitwig-tempo-bridge.md`
  - `DAW Port` host parameter and Bitwig extension rendezvous.

## Current Parameter Groups

- GUI/display:
  - `interpolation_duration`
  - `interpolation_curve`
  - Macro migration complete.
- Static BPM model:
  - `bpm_center`
  - `bpm_range`
  - `sample_rate`
  - Macro migration complete.
- Normal distribution:
  - `std_dev`
  - `factor`
  - `cutoff`
  - `resolution`
  - Macro migration complete.
- Dynamic scoring:
  - `beats_lookback`
  - `normal_distribution_weight`
  - `time_distance_weight`
  - `velocity_current_note_weight`
  - `velocity_note_from_weight`
  - `in_beat_range_weight`
  - `multiplier_weight`
  - `subdivision_weight`
  - `octave_distance_weight`
  - `pitch_distance_weight`
  - `high_tempo_bias_weight`
- Parameter-like runtime/host state outside `Parameter`:
  - plugin `send_tempo`
  - desktop `midi.send_tempo`
  - desktop `midi.enable_midi_clock`
  - desktop `midi.device_name`
  - plugin `daw_port`

## Visitor Consumers And Manual Lists

- Homogeneous generated visitor consumers:
  - `rust/crates/gui/src/add_slider.rs`: `SlideAdder` uses the generated generic `parameter(...)` fallback for
    `GUIParameterVisitor`, covering `interpolation_duration` then `interpolation_curve`.
  - `rust/crates/gui/src/add_slider.rs`: `SlideAdder` uses the generated generic `parameter(...)` fallback for
    `StaticBPMDetectionParameterVisitor`, covering `bpm_center`, `bpm_range`, then `sample_rate`.
  - `rust/crates/gui/src/add_slider.rs`: `SlideAdder` uses the generated generic `parameter(...)` fallback for
    `NormalDistributionParameterVisitor`, covering `std_dev`, `resolution`, `cutoff`, then `factor`.
- Heterogeneous explicit-field visitor consumers:
  - `rust/crates/gui/src/add_slider.rs`: `SlideAdder` keeps explicit dynamic visitor methods because
    `beats_lookback` uses the plain slider path and the remaining dynamic weights use `add_on_off(...)`.
  - `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`: `DynamicRemoteControlParams` keeps explicit
    dynamic visitor methods because each field maps to a concrete `PluginDynamicParams` host handle.
  - `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`: `DynamicHostConfigReader` keeps explicit dynamic
    visitor methods because host params have different read paths for `IntParam` and `PluginOnOffParam`.
- Order-sensitive manual lists:
  - `rust/crates/gui/src/config_ui.rs`: settings panel order is desktop controls, GUI/display traversal, static traversal,
    normal distribution traversal, dynamic traversal, then `Send tempo`.
  - `rust/crates/midi-bpm-detector-plugin/src/lib.rs`: remote controls expose `Send tempo`, static order
    `bpm_center`, `bpm_range`, `sample_rate`, normal order `std_dev`, `resolution`, `cutoff`, `factor`, then dynamic
    generated traversal.
  - Generated normal-distribution traversal order is `std_dev`, `resolution`, `cutoff`, `factor`, matching the canonical
    GUI settings order.
- Leave-alone bespoke runtime/host mappings:
  - `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`: `MidiBpmDetectorParams::new` manually constructs
    host params so IDs, callbacks, nested groups, and concrete `nih-plug` handles stay visible.
  - `rust/crates/midi-bpm-detector-plugin/src/task_executor.rs`: host-origin static/normal copy-back and GUI/display
    dynamic-task copy-back encode update timing and GUI refresh behavior, not just traversal.
  - `rust/crates/midi-bpm-detector-plugin/src/bpm_detector_configuration.rs`: `LiveConfig` setters write config, host
    params, and delayed update markers together; treat that as synchronization policy until a dedicated slice reviews it.
- Future helper candidates:
  - repeated plugin adapter calls may justify a helper only after host-visible ordering and field-to-handle visibility are
    preserved explicitly;
  - test-only label visitors in `gui/src/config.rs` and `bpm_detection_core/src/parameters.rs` can be revisited if a later
    slice changes visitor exhaustiveness/default-method policy.

## Boundaries To Preserve

- Do not move egui/UI dependencies into `bpm_detection_core`, `bpm_detection_midi`, or lower-level algorithm crates.
- Do not move plugin/nih-plug dependencies into shared core/gui crates.
- Do not let a parameter macro decide runtime synchronization policy. Static/dynamic/gui/output update semantics must
  remain explicit at runtime-specific call sites.
- Do not broaden the Bitwig tempo bridge or `DAW Port` semantics while working on parameter declaration mechanics.
- Do not introduce macros in Rust code unless the user explicitly agreed. For this audit, the user did explicitly agree
  that a macro is justified for the mechanical parameter group pattern.

## Useful Checks

Run Cargo commands from `rust/`.

Narrow checks for the dynamic macro and metadata-spec slices:

```sh
cargo test -p parameter
cargo test -p parameter_macros
cargo test -p bpm_detection_core parameter_inventory_tests
cargo test -p bpm_detection_core
cargo test -p gui
cargo test -p midi-bpm-detector-plugin
```

Broader checks if the slice touches runtime wrappers or public cross-crate APIs:

```sh
cargo test -p desktop
cargo test -p wasm --target wasm32-unknown-unknown
cargo clippy -p desktop -p midi-bpm-detector-plugin -p midi-reset --all-targets
```

The wasm target may need local setup; if unavailable, the implementer should record that in the back-handoff.

## Open Questions

- Should a generated visitor trait keep default no-op methods for compatibility, or should the macro tighten visitor
  exhaustiveness in a later slice?
- Should static, normal, and GUI groups eventually get visitors, or should the macro produce a simpler typed enumeration
  API that replaces visitors?
- For visitor consumers, prefer the generated generic `parameter(...)` fallback when every visited parameter has the same
  behavior. Keep explicit field methods when parameter types, host handles, or side effects differ.
- Normal-distribution generated traversal, GUI settings, plugin parameter construction, plugin copy-back, and plugin
  CLAP remote controls now use `std_dev`, `resolution`, `cutoff`, `factor`. This is the canonical user-facing order.
- Should the static computed-method extension remain public after the static macro migration, or should GUI histogram
  code eventually call inherent/static helper methods directly?
- Should output/runtime state such as `send_tempo` become part of a typed parameter catalog, or remain explicitly bespoke
  because of realtime/host differences?
