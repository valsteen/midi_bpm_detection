# Plugin Flow

This document describes the CLAP/VST3 plugin path. The plugin is the production mode and the strictest runtime boundary.

## Committed Parameter Path

`MidiBpmDetectorParams` holds the plugin's committed host parameters and the persisted enable bits attached to `OnOff`
parameters. Host automation and host-parameter editor gestures change nice-plug parameters. An enabled-only `OnOff`
gesture changes its persisted adapter bit without a host gesture because that bit is not exposed as a second automatable
parameter. It does not independently mark the dynamic group; the detector observes it when a later dynamic parameter
callback schedules another configuration task. Parameter-backed detector configuration follows this path:

```text
host automation or host-parameter editor gesture
    -> CLAP parameter value and callback
    -> per-group DeferredConfigUpdate sample marker
    -> MidiBpmDetector::process audio-block boundary
    -> concrete Task payload
    -> TaskExecutor-owned BPMDetection
```

Static, dynamic, and GUI parameter groups each have one atomic first-change marker. A callback records the current sample
only while its group is idle, preserving the start of the coalescing window. `send_tempo` and `daw_port` are focused
outputs instead: their callbacks update dedicated atomics and do not configure the detector.

## Editor Reconciliation and Commit

`PluginGuiEditor` owns a local `EditableSettings` draft and the previous committed host snapshot. On every editor update,
`merge_host_changes` compares consecutive host snapshots field by field:

- a field whose host value changed replaces that field in the draft;
- a host field that did not change leaves the current draft value intact;
- nested static configuration is reconciled with the same field-wise rule;
- `send_tempo` is reconciled independently from the three BPM setting groups.

This preserves an editor proposal while nice-plug has not yet reflected it in parameter readback, without allowing stale
draft values to overwrite newer host automation.

The editor calls `BPMDetectionGUI::prepare`, then passes nice-plug's supplied `&mut egui::Ui` and its draft to
`BPMDetectionGUI::show`. The resulting `GuiChanges` receipt gates commit work by group. Generated
`MirrorChangedConfig` implementations then compare individual fields within each edited group. Changed host-parameter
values receive `ParamSetter` begin/set/end gestures. For an `OnOff` value, the numeric portion uses its `ParamSetter`;
changing only the persisted enable bit updates the adapter state without creating a host gesture or dirty-group marker.

The plugin-only `T` shortcut toggles `draft.send_tempo` and sets the same `GuiChanges::send_tempo` flag as the visible
control. Both routes use the same explicit `ParamSetter` gesture. Shared GUI rendering never writes a host parameter
directly.

## Audio-Block Coalescing

The host calls `MidiBpmDetector::process` with a finite audio buffer and the MIDI events scheduled inside it. After a
50 ms sample-clock window, each mature dirty marker produces one concrete background task:

- `Task::ApplyStaticConfig(StaticBPMDetectionConfig)` rebuilds detector model state and recomputes;
- `Task::ApplyDynamicConfig(DynamicBPMDetectionConfig)` replaces the executor's scoring settings and recomputes;
- `Task::RefreshGui` asks an open editor to repaint.

The task enum carries fixed-shape values rather than a trait-object payload. Parameter changes do not use a second
editor-owned detector configuration, lock, delay, or dispatch path.

## Buffer-Oriented MIDI Processing

MIDI timing remains tied to the current host block:

```text
host process block
    -> read transport tempo
    -> drain this block's MIDI events
    -> convert note-on timing to an absolute sample timestamp
    -> try_push Event values into a fixed ring buffer
    -> schedule Task::ProcessNotes when work exists
```

`buffer.samples()` advances the plugin's absolute sample clock. `event.timing()` locates each event within the block, and
`sample_to_duration` produces the timestamp consumed by `BPMDetection`. Host events are also sent onward through the
plugin context.

Project-owned storage and handoffs on this process path are fixed-size or atomic. The callback contains no explicit heap
allocation, lock acquisition, file/network I/O, waiting, detector computation, or self-driven retry loop. Its event loop
is bounded by the host's finite event set for the current block. `try_push` keeps ring-buffer pressure nonblocking; a full
ring logs and drops that publication rather than waiting.

## Background Executor Ownership

nice-plug calls one mutable task-executor closure that captures a concrete `TaskExecutor`. Its `DetectionRuntime`
exclusively owns:

- `BPMDetection`;
- the current dynamic detection configuration;
- the fixed event-ring consumer.

`Task::ProcessNotes` drains note and DAW-tempo events. Static and dynamic task payloads mutate the same owned detector
runtime. BPM computation, histogram publication, and optional localhost tempo-controller I/O occur in this background
executor, outside `MidiBpmDetector::process`.

## Display Publication

Opening the editor creates `BPMDetectionGUI`, `BpmDisplayPublisher`, and `GuiContextHandle`. A one-slot handoff gives the
two weak capabilities to `TaskExecutor`; closing the editor releases its live copies.

After a successful BPM computation, `BpmDisplayPublisher` stores the scalar estimate and attempts to swap the histogram
into the GUI's latest snapshot. The publisher reuses preallocated scratch storage, never waits for the renderer, and drops
the visualization update on contention. Once the GUI is dropped, publication and repaint requests are no-ops.

## Tempo Feedback

The plugin cannot act as a system MIDI device or MIDI clock provider. `TempoControllerOutput` optionally sends detected
BPM from the background executor to the localhost controller bridge; no socket operation occurs in the realtime callback.
The bridge contract is documented in [Bitwig tempo bridge](bitwig-tempo-bridge.md).
