# Parameter Flow Audit

This audit tracks user-editable algorithm, GUI, plugin, and MIDI-output parameters before changing the parameter
architecture. It starts with declaration inventory, then traces how each parameter group moves through config loading,
egui, host parameters, runtime tasks, worker snapshots, and atomic side channels.

## Declaration Sources

- `rust/crates/parameter/src/lib.rs`: generic `Parameter<Config, ValueType>` metadata: label, unit, range, step,
  logarithmic flag, default, getter, and setter.
- `rust/crates/bpm_detection_core/src/parameters.rs`: static BPM, dynamic BPM, and normal distribution config structs,
  accessor traits, typed `Parameter` constants, defaults, validation, and the dynamic parameter visitor.
- `rust/crates/gui/src/config.rs`: GUI/display config struct, accessor trait, typed `Parameter` constants, defaults, and
  validation.
- `rust/crates/bpm_detection_midi/src/lib.rs`: desktop MIDI service config, including atomic output flags.
- `rust/crates/midi-bpm-detector-plugin/src/plugin_config.rs`: plugin load-time config shape.
- `rust/crates/midi-bpm-detector-plugin/src/plugin_parameters.rs`: host-facing plugin parameter structs and manual
  construction from typed config parameters.

## Typed Parameter Inventory

### GUI / Display

These parameters affect visual interpolation, not BPM scoring or model shape.

| Field | Type | Label | Range | Step | Log | Code default | Declared in |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `gui_config.interpolation_duration` | `Duration` | `Interpolation duration` | `0.050..=1.0` seconds | `0.0` | no | `500 ms` | `GUIParameters::INTERPOLATION_DURATION` |
| `gui_config.interpolation_curve` | `f32` | `Interpolation curve` | `0.1..=2.0` | `0.0` | no | `0.7` | `GUIParameters::INTERPOLATION_CURVE` |

Inventory note: desktop, plugin, and wasm shipped base configs all override these code defaults.

### Static BPM Model Shape

These parameters define the BPM search space and sample resolution. They require static model/buffer updates.

| Field | Type | Label | Range | Step | Log | Code default | Declared in |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `static_bpm_detection_config.bpm_center` | `f32` | `BPM center` | `1.0..=150.0` | `0.01` | no | `90.0` | `StaticBPMDetectionParameters::BPM_CENTER` |
| `static_bpm_detection_config.bpm_range` | `u16` | `BPM range` | `1.0..=100.0` | `1.0` | no | `40` | `StaticBPMDetectionParameters::BPM_RANGE` |
| `static_bpm_detection_config.sample_rate` | `u16` | `BPM sample rate` | `1.0..=10000.0` samples/second | `1.0` | yes | `450` | `StaticBPMDetectionParameters::SAMPLE_RATE` |

Inventory note: desktop, plugin, and wasm shipped base configs all override at least `bpm_center` and `sample_rate`.

### Normal Distribution Static Submodel

These parameters are nested under `StaticBPMDetectionConfig`. They affect the precomputed normal distribution model and
therefore belong to the static update path.

| Field | Type | Label | Range | Step | Log | Code default | Declared in |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `static_bpm_detection_config.normal_distribution.std_dev` | `f64` | `Standard deviation` | `4.0..=40.0` | `0.0` | no | `24.0` | `NormalDistributionParameters::STD_DEV` |
| `static_bpm_detection_config.normal_distribution.factor` | `f32` | `factor` | `0.0..=50.0` | `0.0` | no | `40.0` | `NormalDistributionParameters::FACTOR` |
| `static_bpm_detection_config.normal_distribution.cutoff` | `f32` | `Normal distribution cutoff` | `1.0..=2000.0` ms | `0.0` | yes | `100.0` | `NormalDistributionParameters::CUTOFF` |
| `static_bpm_detection_config.normal_distribution.resolution` | `f32` | `Normal distribution resolution` | `0.01..=1000.0` ms | `0.0` | yes | `0.6` | `NormalDistributionParameters::RESOLUTION` |

Inventory note: all shipped base configs include explicit normal distribution values, and those values differ by runtime.

### Dynamic BPM Scoring

These parameters affect runtime scoring/evaluation and are currently grouped behind `DynamicBPMDetectionParameters::visit`.

| Field | Type | Label | Range | Step | Log | Code default | Declared in |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `dynamic_bpm_detection_config.beats_lookback` | `u8` | `Beats Lookback` | `2.0..=32.0` | `1.0` | no | `8` | `DynamicBPMDetectionParameters::BEATS_LOOKBACK` |
| `dynamic_bpm_detection_config.normal_distribution_weight` | `OnOff<f32>` | `Normal distribution` | `0.0..=1.0` | `0.0` | no | `On(1.0)` | `DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION_WEIGHT` |
| `dynamic_bpm_detection_config.time_distance_weight` | `OnOff<f32>` | `Time distance` | `0.5..=6.0` | `0.0` | yes | `On(0.7)` | `DynamicBPMDetectionParameters::TIME_DISTANCE_WEIGHT` |
| `dynamic_bpm_detection_config.velocity_current_note_weight` | `OnOff<f32>` | `Note velocity` | `0.5..=10.0` | `0.0` | yes | `On(0.7)` | `DynamicBPMDetectionParameters::VELOCITY_CURRENT_NOTE_WEIGHT` |
| `dynamic_bpm_detection_config.velocity_note_from_weight` | `OnOff<f32>` | `From note velocity` | `0.5..=10.0` | `0.0` | yes | `On(0.7)` | `DynamicBPMDetectionParameters::VELOCITY_NOTE_FROM_WEIGHT` |
| `dynamic_bpm_detection_config.in_beat_range_weight` | `OnOff<f32>` | `In beat range` | `0.0..=3.0` | `0.0` | no | `On(0.75)` | `DynamicBPMDetectionParameters::IN_BEAT_RANGE_WEIGHT` |
| `dynamic_bpm_detection_config.multiplier_weight` | `OnOff<f32>` | `Multiplier` | `0.0..=3.0` | `0.0` | no | `On(0.66)` | `DynamicBPMDetectionParameters::MULTIPLIER_WEIGHT` |
| `dynamic_bpm_detection_config.subdivision_weight` | `OnOff<f32>` | `Subdivision` | `0.5..=6.0` | `0.0` | yes | `On(0.7)` | `DynamicBPMDetectionParameters::SUBDIVISION_WEIGHT` |
| `dynamic_bpm_detection_config.octave_distance_weight` | `OnOff<f32>` | `Octave distance` | `0.5..=20.0` | `0.0` | yes | `On(0.6)` | `DynamicBPMDetectionParameters::OCTAVE_DISTANCE_WEIGHT` |
| `dynamic_bpm_detection_config.pitch_distance_weight` | `OnOff<f32>` | `Pitch distance` | `0.5..=20.0` | `0.0` | yes | `On(0.6)` | `DynamicBPMDetectionParameters::PITCH_DISTANCE_WEIGHT` |
| `dynamic_bpm_detection_config.high_tempo_bias_weight` | `OnOff<f32>` | `High tempo bias` | `0.0..=3.0` | `0.0` | no | `On(0.2)` | `DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS_WEIGHT` |

Inventory notes:

- The code default for `beats_lookback` is duplicated: `DynamicBPMDetectionConfig::default` writes `8` directly instead
  of reading `DefaultDynamicBPMDetectionParameters::BEATS_LOOKBACK.default`.
- The visitor currently gives dynamic parameters a single typed traversal used by validation, egui rendering, remote
  controls, and plugin host-to-config reads.
- All shipped base configs include explicit dynamic values, and many override both enabled state and numeric value.

## Non-`Parameter` User/Runtime State

These items are user visible or host visible, but are not declared with the generic `Parameter` type.

| Field / item | Type | Domain | Declared in | Current modeling |
| --- | --- | --- | --- | --- |
| `send_tempo` in plugin config | `ArcAtomicBool` | Tempo output | `PluginConfig` | Serialized config plus plugin `BoolParam`; mirrored through atomics and GUI write-through |
| `midi.send_tempo` in desktop config | `ArcAtomicBool` | Tempo output | `MidiServiceConfig` | Serialized config plus egui toggle; read by MIDI worker |
| `midi.enable_midi_clock` in desktop config | `ArcAtomicBool` | MIDI clock output | `MidiServiceConfig` | Serialized config; read by MIDI worker |
| `midi.device_name` in desktop config | `String` | Desktop MIDI input selection | `MidiServiceConfig` | Serialized config plus desktop-specific egui selector |
| `daw_port` in plugin params | `ArcAtomicOptionNonZeroU16` plus host `IntParam` | Plugin DAW tempo bridge | `MidiBpmDetectorParams` / plugin runtime | Host parameter stores a pending port in an atomic option; not part of `PluginConfig` |

Inventory notes:

- `send_tempo` is conceptually shared between GUI and host/plugin output, but each runtime models it separately from the
  typed parameter metadata.
- `daw_port` is host-visible and user-editable in plugin hosts, but it is runtime-only and not part of the persisted
  plugin config.
- GUI telemetry and handoff state such as histogram buffers, estimated BPM, DAW BPM, repaint handles, and
  `force_evaluate_bpm_detection` are intentionally excluded from this parameter inventory unless later flow tracing shows
  they participate in user-editable config.

## Shipped Config Files

The repo has separate built-in TOML configs:

- `rust/crates/desktop/config/base_config.toml`
- `rust/crates/midi-bpm-detector-plugin/config/base_config.toml`
- `rust/crates/wasm/config/base_config.toml`

Each shipped config declares the GUI, static, normal distribution, and dynamic parameter groups explicitly. The plugin and
wasm configs also include top-level `send_tempo`; the desktop config includes `send_tempo` under `[MIDI]` alongside
desktop-only MIDI service state.

## Flow Trace

This section maps how each parameter group moves from defaults/config into UI, host parameters, runtime tasks, workers,
and atomic side channels. It does not yet decide whether the flow is correct; that belongs to the invariant pass.

### Shared egui Surface

All runtimes use `BPMDetectionGUI::settings_panel` as the shared parameter UI.

The settings panel currently renders controls in this order:

1. runtime-specific desktop controls through `config.desktop_controls(ui)`;
2. GUI/display sliders for interpolation duration and curve;
3. static BPM sliders for center/range/sample rate;
4. normal distribution sliders;
5. all dynamic scoring controls through `DynamicBPMDetectionParameters::visit(&mut slide_adder)`;
6. a plain `Send tempo` toggle through `BPMDetectionConfig::get_send_tempo` / `set_send_tempo`.

`SlideAdder` reads and writes through each typed `Parameter` getter/setter. For `OnOff<T>` values, the optional
`on_off_widgets` feature adds an enable checkbox; otherwise the widget shows only the numeric slider and treats the
parameter as enabled for editing.

### Desktop Runtime

Desktop loads `DesktopConfig` from built-in TOML plus an optional user config file. It validates GUI, static, and dynamic
config groups before starting the runtime.

Startup flow:

```text
DesktopConfig::new()
  -> start_desktop_controller(config)
  -> MidiService::new(midi config, static config, dynamic config, GuiRemote)
  -> worker::spawn(...)
  -> build_gui_config(config, controller, command queue)
  -> DesktopBaseConfig
  -> shared egui settings panel
```

Desktop egui setter flow:

| Group | Setter owner | Immediate mutation | Propagation path | Worker/runtime effect |
| --- | --- | --- | --- | --- |
| GUI/display | `DesktopBaseConfig: GUIConfigAccessor` | `config.gui_config` | `propagate_dynamic_changes()` | sends current `DynamicBPMDetectionConfig` to MIDI worker |
| Static BPM | `DesktopBaseConfig: StaticBPMDetectionConfigAccessor` | `config.static_bpm_detection_config` | `propagate_static_changes()` | queues `BpmWorkerCommand::StaticBPMDetectionConfig` |
| Normal distribution | `DesktopBaseConfig: NormalDistributionConfigAccessor` | nested static config | `propagate_static_changes()` | queues `BpmWorkerCommand::StaticBPMDetectionConfig` |
| Dynamic scoring | `DesktopBaseConfig: DynamicBPMDetectionConfigAccessor` | `config.dynamic_bpm_detection_config` | `propagate_dynamic_changes()` | queues `BpmWorkerCommand::DynamicBPMDetectionConfig` |
| `send_tempo` | `DesktopBaseConfig: BPMDetectionConfig` | `midi.send_tempo` atomic | no queued config command | MIDI worker reads atomic when BPM is computed |
| `enable_midi_clock` | desktop MIDI config | atomic | no shared settings-panel control in the generic parameter list | MIDI output thread polls atomic |
| `device_name` | desktop MIDI config / device selector | controller/device selection state | controller command queue | reconnects MIDI input listener |

Desktop worker flow:

- Static updates are debounced by the native MIDI worker, then `BPMDetection::update_static_config` rebuilds model state.
- Dynamic updates replace the worker's `dynamic_bpm_detection_config` snapshot and schedule an evaluation.
- Computed BPM updates the MIDI clock interval atomic and, when `send_tempo` is true, emits a tempo output command.
- `send_tempo` and `enable_midi_clock` stay outside the `Parameter` abstraction and are shared with worker threads through
  atomics.

Trace note: desktop GUI/display setters currently use the dynamic propagation callback even though the interpolation
fields are not part of `DynamicBPMDetectionConfig`.

### wasm Runtime

Wasm loads `WASMConfig` from built-in TOML and creates a `BaseConfig` for the shared egui UI.

Startup flow:

```text
WASMConfig::default()
  -> BaseConfig::new(redraw_sender)
  -> create_gui(live_config)
  -> browser task owns BPMDetection + dynamic config snapshot
```

Wasm egui setter flow:

| Group | Setter owner | Immediate mutation | Propagation path | Worker/runtime effect |
| --- | --- | --- | --- | --- |
| GUI/display | `BaseConfig: GUIConfigAccessor` | `config.gui_config` | `propagate_dynamic_changes()` | sends current `DynamicBPMDetectionConfig` queue item |
| Static BPM | `BaseConfig: StaticBPMDetectionConfigAccessor` | `config.static_bpm_detection_config` | `QueueItem::StaticParameters` | delayed static update rebuilds `BPMDetection` |
| Normal distribution | `BaseConfig: NormalDistributionConfigAccessor` | nested static config | `QueueItem::StaticParameters` | delayed static update rebuilds `BPMDetection` |
| Dynamic scoring | `BaseConfig: DynamicBPMDetectionConfigAccessor` | `config.dynamic_bpm_detection_config` | `QueueItem::DynamicParameters` | delayed dynamic update replaces scoring snapshot |
| `send_tempo` | `BaseConfig: BPMDetectionConfig` | no-op / always false | none | not used in wasm |

Wasm runtime flow:

- Static and dynamic queue items are coalesced with a 200 ms browser-side delay.
- Notes and dynamic changes share the delayed dynamic redraw/evaluation path.
- There is no host parameter surface and no tempo-output side channel in the shared wasm config path.

Trace note: like desktop, wasm GUI/display setters currently propagate a dynamic config queue item even though the GUI
fields do not participate in BPM scoring.

### Plugin Runtime

Plugin startup loads `PluginConfig::default()` from built-in TOML, constructs host-facing `MidiBpmDetectorParams`, then
shares the config with the task executor and optional egui editor through an `Arc<RwLock<PluginConfig>>`.

Startup flow:

```text
PluginConfig::default()
  -> BPMDetection::new(static config)
  -> MidiBpmDetectorParams::new(config, deferred static marker, deferred dynamic marker, current_sample, daw_port)
  -> TaskExecutor owns BPMDetection + dynamic config snapshot + params + shared config
  -> GuiEditor owns shared config + params + GUI handoff state
```

Plugin host parameter construction:

| Group | Host params | Callback marker | Construction style |
| --- | --- | --- | --- |
| GUI/display | `PluginGUIParams` | dynamic deferred marker | manual fields built from `GUIParameters` |
| Static BPM | `PluginStaticParams` | static deferred marker | manual fields built from `StaticBPMDetectionParameters` |
| Normal distribution | nested `NormalDistributionParams` | static deferred marker | manual fields built from `NormalDistributionParameters` |
| Dynamic scoring | `PluginDynamicParams` | dynamic deferred marker | manual fields built from `DynamicBPMDetectionParameters`; dynamic traversal is reused for remote controls and host reads |
| `send_tempo` | top-level `BoolParam` | direct atomic store callback | not a typed `Parameter` |
| `daw_port` | top-level `IntParam` | direct atomic option store callback | not part of `PluginConfig` |

Plugin host-origin flow:

```text
DAW changes host param
  -> param callback marks static or dynamic DeferredConfigUpdate at current_sample
  -> process() waits HOST_PARAMETER_SYNC_COALESCING_WINDOW on sample clock
  -> context.execute_background(Task::*Config(ParameterSyncOrigin::Host))
  -> TaskExecutor copies host param values into shared PluginConfig
  -> gui_must_update_config = true
  -> TaskExecutor updates BPMDetection/static snapshot or dynamic snapshot
  -> forced ProcessNotes recompute
```

Host-origin group details:

- Static BPM and normal distribution fields are copied manually from `self.params.static_params` into
  `config.static_bpm_detection_config`, then applied to `BPMDetection`.
- Dynamic scoring fields are copied through `PluginDynamicParams::read_dynamic_config()`, which uses the dynamic visitor.
- GUI/display fields are copied inside the dynamic task. `interpolation_duration` is assigned twice in the current code
  path, and both GUI fields share the dynamic deferred marker.
- `send_tempo` is copied from the top-level host `BoolParam` into the shared atomic during the dynamic task.
- `daw_port` is consumed separately at the beginning of every `TaskExecutor::execute` call by taking the pending atomic
  option and reconnecting the tempo controller socket.

Plugin GUI-origin flow:

```text
egui editor opens
  -> GuiEditor::build clones shared PluginConfig into BaseConfig
  -> shared egui settings panel edits LiveConfig
  -> LiveConfig setter mutates local PluginConfig clone
  -> LiveConfig setter writes through matching host param with ParamSetter
  -> BaseConfig delays static or dynamic update for GUI_PARAMETER_SYNC_COALESCING_WINDOW
  -> delayed update writes local config clone into shared PluginConfig
  -> async background Task::*Config(ParameterSyncOrigin::Gui)
  -> TaskExecutor reads shared PluginConfig and updates BPMDetection/static or dynamic snapshot
```

Plugin GUI-origin group details:

| Group | LiveConfig setter side effects | Delayed task |
| --- | --- | --- |
| GUI/display | update local `gui_config`, write matching host GUI param, call `delay_dynamic_changes()` | dynamic task reads shared config, updates dynamic scoring snapshot only |
| Static BPM | update local static config, write matching host static param, call `delay_static_changes()` | static task updates `BPMDetection` from shared config |
| Normal distribution | update nested static config, write matching host param, call `delay_static_changes()` | static task updates `BPMDetection` from shared config |
| Dynamic scoring | update local dynamic config, write matching host dynamic/on-off param, call `delay_dynamic_changes()` | dynamic task updates dynamic scoring snapshot |
| `send_tempo` | store atomic and mark `send_tempo_changed` | editor update writes top-level `BoolParam` through `ParamSetter` |

Plugin remote controls:

- Remote controls expose `send_tempo`, static BPM, normal distribution, and dynamic scoring params.
- Dynamic remote controls use `PluginDynamicParams::add_remote_controls()`, which traverses
  `DefaultDynamicBPMDetectionParameters::visit`.
- GUI/display params and `daw_port` are host params but are not currently included in the CLAP remote-control sections.

## Invariant And Failure-Mode Audit

This section records the current guardrails and the places where the code can still drift without a compile-time error.

### Config Shape And Value Validity

Guarded today:

- GUI, static BPM, dynamic BPM, normal distribution, plugin config, wasm config, and MIDI service config use
  `deny_unknown_fields` at their own serde boundaries.
- Plugin, desktop, and wasm config loaders validate GUI, static, and dynamic config groups after deserialization.
- Validation uses the typed `Parameter` ranges, so config-file values outside declared ranges are rejected.
- `OnOff<T>` deserialization rejects unknown keys inside `{ enabled, value }` maps, and validation checks the stored
  numeric value regardless of enabled state.
- Plugin and wasm built-in config failures panic at startup rather than letting an invalid built-in config run.

Drift risks:

- `DesktopConfig` itself does not use `deny_unknown_fields`, even though several nested structs do. Unknown top-level
  desktop config keys are therefore not guarded in the same way as plugin/wasm config keys.
- Runtime setters generally rely on egui sliders or host parameter ranges to keep values valid. There is no shared
  post-set validation step after every UI/host edit.
- Code defaults and shipped TOML defaults are separate sources. Most struct defaults read from `Parameter` constants, but
  shipped runtime defaults override many values.
- `DynamicBPMDetectionConfig::default` duplicates `beats_lookback = 8` instead of reading
  `DefaultDynamicBPMDetectionParameters::BEATS_LOOKBACK.default`.

### Mapping Completeness

Guarded today:

- Accessor traits force each runtime config wrapper to implement getters/setters for the declared static, normal,
  dynamic, and GUI fields.
- `DynamicBPMDetectionParameters::visit` gives dynamic scoring a shared traversal used by validation, egui rendering,
  plugin remote controls, and plugin host-to-config reads.
- Tests cover the dynamic visitor label order, `SlideAdder` implementing the dynamic visitor, dynamic remote-control
  exposure, dynamic host-param reads back into `DynamicBPMDetectionConfig`, and dynamic on/off persistent key names.
- Tests cover selected plugin parameter initial values and a small static host-param ID check.

Drift risks:

- The dynamic visitor trait methods have default implementations. If a new dynamic parameter gets a default visitor
  method, visitors can compile while silently doing nothing for that parameter.
- Static BPM, normal distribution, GUI/display, `send_tempo`, `enable_midi_clock`, `device_name`, and `daw_port` do not
  have an equivalent shared traversal.
- The shared egui panel manually lists GUI/static/normal groups, then uses the visitor only for dynamic scoring. Forgetting
  to add a non-dynamic parameter to egui would not be caught by the type system.
- Plugin host parameter construction manually lists every GUI/static/normal/dynamic/output field. The dynamic group gets
  some visitor reuse later, but construction itself is still a hand-maintained field list.
- Plugin host-origin static and normal distribution sync manually copies each host param back into `PluginConfig`.
  Dynamic scoring uses `read_dynamic_config()`, but GUI/display and `send_tempo` are copied inside the dynamic task.
- Remote controls are another hand-maintained exposure surface: dynamic controls use the visitor, but `send_tempo`,
  static, and normal controls are listed manually, while GUI/display params and `daw_port` are omitted.

### Update-Path Semantics

Guarded today:

- Static and dynamic algorithm changes are separated in desktop and wasm runtime messages.
- Plugin host-origin updates are tagged with `ParameterSyncOrigin::Host`; plugin GUI-origin updates are tagged with
  `ParameterSyncOrigin::Gui`.
- Plugin host-origin changes are coalesced on the host sample clock with `HOST_PARAMETER_SYNC_COALESCING_WINDOW`.
- Plugin GUI-origin changes are coalesced on the editor wall clock with `GUI_PARAMETER_SYNC_COALESCING_WINDOW`.
- `DeferredConfigUpdate::mark_changed_at_if_idle` preserves the first pending host-change sample until the update is
  taken, and this behavior is tested through the atomic option wrapper and plugin tests.

Drift risks:

- GUI/display interpolation fields ride the dynamic update path in desktop, wasm, and plugin modes, even though they are
  not part of `DynamicBPMDetectionConfig`.
- In plugin host-origin sync, GUI/display fields and `send_tempo` are copied during `Task::DynamicBPMDetectionConfig`.
  This makes the dynamic task mean “dynamic scoring plus GUI/display plus tempo-output host sync”.
- In plugin GUI-origin sync, GUI/display setters write through host params and delay a dynamic task, but the dynamic task
  only updates the dynamic scoring snapshot. The GUI config is still shared for future GUI reads, but no algorithm state
  consumes it.
- In plugin host-origin dynamic sync, `interpolation_duration` is assigned twice.
- Desktop and wasm GUI/display edits currently trigger dynamic worker/queue traffic even though the dynamic config payload
  is unchanged.
- `send_tempo` follows different synchronization models by runtime: desktop egui writes an atomic directly, plugin host
  writes a `BoolParam` callback that stores an atomic, and plugin GUI writes a local atomic plus a separate
  `send_tempo_changed` flag so `ParamSetter` can mirror the host param.

### Realtime And Concurrency Boundaries

Guarded today:

- Plugin audio processing avoids direct config locks for parameter sync; host param callbacks mark deferred updates, and
  `process()` schedules background tasks after coalescing.
- Plugin note events use a fixed-size ring buffer and `try_push`.
- Desktop MIDI output flags use atomics read by worker/output threads rather than blocking config locks.
- `daw_port` uses an atomic option handoff and reconnects in the background task executor, not directly inside the host
  parameter callback.
- GUI histogram/BPM state uses atomics or `AtomicRefCell` handoffs, outside the user-editable parameter inventory.

Drift risks:

- Several atomics are parameter-like state (`send_tempo`, `enable_midi_clock`, `daw_port`) but are not described by the
  typed `Parameter` metadata, so their validation, UI exposure, persistence, and sync semantics are bespoke.
- `ArcAtomicBool` ordering differs by path (`Relaxed`, `Acquire`, `Release`, `SeqCst`) without a parameter-level policy.
  This may be fine for the current uses, but it is not captured as an invariant.
- `daw_port` uses `0` as disabled through `NonZeroU16::new`, so host value `0` intentionally becomes `None`. That rule is
  tested at the atomic codec level, but not documented at the host parameter declaration site.
- `PluginOnOffParam` stores enabled state separately from the numeric `FloatParam`. This is necessary for host parameter
  shape, but it means one logical dynamic parameter is split across two persistence/state channels.

### Current Guardrail Summary

| Concern | Current guard strength | Main remaining risk |
| --- | --- | --- |
| Config-file stale keys | strong for plugin/wasm/nested structs; weaker at desktop top level | unknown desktop top-level keys may slip through |
| Config-file ranges | strong at load time | runtime setter paths do not share a validation checkpoint |
| Dynamic scoring traversal | medium | visitor defaults can hide missing visitor behavior |
| Static/normal/GUI mapping completeness | weak to medium | repeated manual lists can drift |
| Plugin host/config mirror | medium | manual construction and host-origin static/GUI/output copies can drift |
| Host/GUI sync loops | medium | origin tags help, but dynamic task carries several unrelated domains |
| Realtime safety | medium to strong | atomics are appropriate but policy is scattered |

## Findings To Carry Forward

- There are 20 typed `Parameter` declarations: 2 GUI/display, 3 static model, 4 normal distribution static submodel, and
  11 dynamic scoring parameters.
- There are at least 5 user-visible or host-visible runtime fields outside the typed `Parameter` abstraction:
  plugin `send_tempo`, desktop `midi.send_tempo`, `midi.enable_midi_clock`, `midi.device_name`, and plugin `daw_port`.
- Runtime base configs are not simple mirrors of code defaults. The audit must distinguish code defaults from shipped
  runtime defaults.
- Dynamic parameters have the strongest shared traversal. GUI/static/normal/output parameters are more manually wired.
- Flow tracing confirms that GUI/display updates currently ride dynamic update paths in desktop, wasm, and plugin modes.
- The current visitor approach improves dynamic parameter reuse, but its default methods do not fully guard against
  forgotten mappings.
- The next pass should evaluate whether the trait/visitor structure should be tightened, extended to other groups, or
  replaced by a grouped registry/catalog that makes mapping omissions harder.
