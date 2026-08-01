# Architecture

This document is the high-level map of the project and routes to the detailed runtime data flows.

## Purpose

The project estimates the tempo of incoming MIDI notes in realtime. The core algorithm compares intervals between recent
notes, scores likely beat durations, and exposes the most likely BPM plus histogram data for visualization.

The repository has two build roots:

- `rust`: the Cargo workspace for the BPM detector core, plugin, desktop app, WASM demo, and Rust tools.
- `extension`: the Gradle workspace for the Bitwig controller extension that lets the plugin control Bitwig tempo.

The same BPM detection model is used in three Rust operating modes:

- `plugin`: a CLAP/VST3 plugin intended to run inside a DAW. This is the production target.
- `desktop`: a native GUI development app.
- `wasm`: a browser demo using the shared egui UI.

The main architectural goal is to keep these modes from importing unnecessary dependencies from each other. Each mode
owns its host/runtime integration, while shared crates carry the algorithm, configuration shapes, reusable GUI, and small
cross-platform abstractions.

The production Bitwig tempo-control path spans both build roots: the Rust plugin estimates BPM and sends tempo updates
over a localhost bridge, while the Kotlin Bitwig controller extension owns the Bitwig transport-tempo write.

## Terminology

- Note-on event: the core input observation used for BPM detection. It includes timestamp, MIDI channel, pitch, and
  velocity. It is more precise than "note", which can also mean only pitch.
- Timed MIDI message: a runtime MIDI message with a timestamp, kept in native/host-facing crates for display, parsing,
  and protocol handling.
- BPM worker command: a message sent to a background BPM worker. It is already filtered to something the worker can act on,
  such as a note-on event, config change, or transport command.
- MIDI output command: a side effect owned by the native MIDI output thread, such as play, stop, or tempo feedback.
- Static BPM config: settings that reshape the detection model and require buffer/precomputed-data updates.
- Dynamic BPM config: scoring weights and lookback values that can be applied without rebuilding the detection model.

## Build-Root Detail

This page intentionally does not list every Rust crate or Kotlin module. Those inventories live with their build roots:

- [Rust workspace architecture](../rust/architecture.md): crate graph, crate groups, parameter-stack dependency rules,
  plugin realtime constraints, and Rust runtime-mode boundaries.
- `extension/`: the Bitwig controller extension build root. Its repository-facing contract is the tempo bridge described
  in [Bitwig tempo bridge](bitwig-tempo-bridge.md).

At the repository level, the important dependency direction is simpler: Rust owns BPM detection and the plugin runtime,
the Kotlin extension owns Bitwig controller API integration, and the two communicate through a narrow localhost tempo
bridge.

## Operating Mode Boundaries

The same conceptual pipeline appears in each mode:

```text
MIDI/key input -> runtime-specific parsing -> core note events -> BPMDetection -> histogram/BPM output
    -> UI and/or host integration
```

The important difference is where that pipeline is allowed to do work:

- In plugin mode, the audio/plugin callback is the constrained boundary. Project-side work there uses fixed-size values
  and nonblocking handoffs; BPM computation runs in the background executor.
- In desktop mode, MIDI and BPM work can live in native worker threads. The desktop controller bridges the native MIDI
  runtime into the native GUI app without moving those dependencies into the shared GUI layer.
- WASM mode has no native worker threads. Browser events and delayed recomputation are coordinated through async tasks
  and channels.

## Tempo Feedback

Tempo feedback has two runtime-specific implementations:

- Desktop mode can act as a native virtual MIDI device. It emits MIDI clock, play/stop, and small text SysEx messages
  such as `TEMPO|...`. This lets the standalone app communicate with a DAW, with the tradeoff that the DAW depends on an
  external MIDI clock and the host's clock-integration behavior.
- Plugin mode cannot act as a system MIDI device. It runs as a CLAP/VST3 instrument inside the host, so its production
  tempo feedback path is a localhost controller bridge. The plugin sends detected BPM to an external Bitwig controller
  extension, which can set the DAW tempo while still allowing the user to adjust tempo manually.

The native MIDI clock is desktop/experimental support. The localhost controller bridge is the production plugin
integration.

## Communication Direction

The project uses typed peer boundaries rather than a single runtime-wide event bus:

- producers expose narrow capabilities, such as publishing BPM estimates or MIDI device changes;
- consumers depend on those narrow capabilities, not on a whole application event enum;
- shared protocols live at the smallest dependency level that can express the relationship;
- runtime/bootstrap code wires producers and consumers together explicitly;
- after bootstrap, peers communicate through the connection they actually need instead of returning to a universal bus.

These connections are distributed across their owners, so bootstrap records the static runtime graph. Each runtime
connects its concrete producers and consumers directly, and each pair communicates through its focused protocol.

Small explicit enums remain local to narrow, stable protocols. A worker command protocol describes one worker boundary,
not the whole application.

### Communication Boundary Properties

- The core model is independent from runtime dependencies.
- Stable runtime relationships are visible at bootstrap.
- Ownership, thread, async-task, and realtime crossings use explicit typed messages or focused capabilities.
- The plugin realtime ring and shared WASM detector channel use bounded, nonblocking handoffs. Native controller,
  BPM-worker, and MIDI-output queues are unbounded; MIDI-service commands use a zero-capacity synchronous handoff. These
  native channels remain outside the plugin realtime callback.
- Components receive their dependencies during bootstrap rather than locating arbitrary services at runtime.

## Cross-Runtime GUI Invariant

Each runtime supplies an input snapshot and an editable proposal to the shared GUI, then owns the resulting commit:

```text
runtime-owned input snapshot
    -> prepare display and editable state
    -> show UI and collect GuiChanges
    -> concrete runtime-owned commit
```

`BPMDetectionGUI` contains reusable view state and rendering. It has no host parameter handle, desktop controller,
runtime command sender, or persistence owner. Desktop and WASM expose their concrete typed commit boundaries directly.
The plugin maps host-parameter edits to nice-plug `ParamSetter` requests. An `OnOff` field's persisted enable bit is
adapter state rather than a second automatable parameter. Changing only that bit does not create a host request or mark
the dynamic group; the detector observes it only when a later dynamic parameter callback schedules another configuration
task.

## Stable Project Boundaries

- Cargo owns the Rust build root and Gradle owns the Kotlin extension build root; neither build system owns the other.
- Plugin mode is the production target and defines the strict realtime constraints. Detailed Rust-side boundaries live
  in [Rust workspace architecture](../rust/architecture.md) and [Plugin flow](plugin-flow.md).
- Every GUI runtime has an explicit commit boundary, while shared rendering remains free of runtime effects.
- Typed peer boundaries are wired at bootstrap. Runtime-local worker protocols do not form a universal event bus.
- The Rust plugin and Kotlin extension communicate only through the narrow localhost tempo bridge described in
  [Bitwig tempo bridge](bitwig-tempo-bridge.md).
- [Runtime lifecycle](runtime-lifecycle.md) owns the detailed data-flow and thread-boundary diagrams.

## Detailed Flow Notes

- [Runtime lifecycle](runtime-lifecycle.md) documents bootstrap wiring, ownership boundaries, and the main data flows
  after startup across plugin, desktop, and WASM mode.
- [Native MIDI flow](native-midi-flow.md) documents the desktop MIDI service, BPM worker, output thread, and the
  closure-command boundary used by `MidiService::execute()`.
- [Plugin flow](plugin-flow.md) documents host buffer processing, realtime handoff, background BPM work, and plugin
  tempo feedback.
- [Algorithm archaeology](algorithm-archaeology.md) documents the original interval/uncertainty idea, why the histogram
  exists, and why visualization became part of the development loop.
