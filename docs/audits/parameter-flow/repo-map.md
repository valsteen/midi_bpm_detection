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
  - `BPMDetectionGUI::settings_panel` manually lists GUI/static/normal params, then uses the dynamic visitor for dynamic
    scoring params.
  - `SlideAdder` renders typed `Parameter` values and implements `DynamicBPMDetectionParameterVisitor`.

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
- Normal-distribution generated traversal order is `std_dev`, `factor`, `cutoff`, `resolution`, while current GUI order
  is `std_dev`, `resolution`, `cutoff`, `factor` and plugin remote-control order is `resolution`, `factor`, `cutoff`,
  `std_dev`. Decide order semantics before replacing those manual lists.
- Should the static computed-method extension remain public after the static macro migration, or should GUI histogram
  code eventually call inherent/static helper methods directly?
- Should output/runtime state such as `send_tempo` become part of a typed parameter catalog, or remain explicitly bespoke
  because of realtime/host differences?
