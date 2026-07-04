# Architecture

This document is a high-level map of the project. It should help contributors understand the main boundaries before
following the detailed runtime data flows.

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
  in [Bitwig tempo bridge](bitwig-tempo-bridge.md); extension-local agent rules live in
  [extension/AGENTS.md](../extension/AGENTS.md).

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

- In plugin mode, the audio/plugin callback is the constrained boundary. It should not block, allocate, or perform heavy
  BPM computation. It forwards compact events and schedules background work.
- In desktop mode, MIDI and BPM work can live in native worker threads. The desktop controller bridges the native MIDI
  runtime into the native GUI app without moving those dependencies into the shared GUI layer.
- In WASM mode, there are no native worker threads in the current design. Browser events and delayed recomputation are
  coordinated through async tasks and channels.

## Tempo Feedback

Tempo feedback has two historically different implementations:

- Desktop mode can act as a native virtual MIDI device. It can emit MIDI clock, play/stop,
  and small text SysEx messages such as `TEMPO|...`. This was useful for experimenting with a standalone app that could
  still talk to a DAW, but it makes the DAW depend on an external MIDI clock and is ergonomically limited by host clock
  integration.
- Plugin mode cannot act as a system MIDI device. It runs as a CLAP/VST3 instrument inside the host, so its production
  tempo feedback path is a localhost controller bridge. The plugin sends detected BPM to an external Bitwig controller
  extension, which can set the DAW tempo while still allowing the user to adjust tempo manually.

The native MIDI clock code should be read as desktop/experimental support, not as the production plugin integration
strategy.

## Communication Direction

The project should prefer typed peer boundaries over a single runtime-wide event bus. A central bus can be useful early
because it lists "everything that can happen" in one place, but it also tends to become a dependency magnet: the event
enum, dispatcher, and orchestrator eventually need to know about every component.

The preferred direction is:

- producers expose narrow capabilities, such as publishing BPM estimates or MIDI device changes;
- consumers depend on those narrow capabilities, not on a whole application event enum;
- shared protocols live at the smallest dependency level that can express the relationship;
- runtime/bootstrap code wires producers and consumers together explicitly;
- after bootstrap, peers communicate through the connection they actually need instead of returning to a universal bus.

The tradeoff is that connections become more distributed. Bootstrap therefore becomes important documentation: it should
read like clean configuration of the runtime graph, not like a second hidden orchestrator. Pluggable components should
follow the same shape: discover compatible producers and consumers, connect them, then let that pair communicate through
its own protocol.

Small explicit enums are still valid when the protocol is narrow and stable. A worker command protocol belongs to one
worker boundary and should not try to describe the whole application.

### Design Goals For Communication Boundaries

These goals are guidance, not doctrine. They are inspired by existing patterns such as composition root/bootstrap wiring,
ports and adapters, observer/signals, and actor-style worker mailboxes:

- keep the core model independent from runtime dependencies;
- make stable runtime relationships visible at bootstrap;
- prefer small typed protocols over a catch-all application event enum;
- use explicit messages when crossing ownership, thread, async task, or realtime boundaries;
- keep high-volume/realtime paths predictable: bounded work, bounded queues, no accidental blocking;
- avoid service-locator style lookup, where components can silently find anything at runtime;
- document the wiring well enough that distributed peer connections remain understandable.

These choices should be re-evaluated as the architecture becomes clearer. A central enum, bus, or dispatcher can still
be the right tool for a narrow UI loop, worker mailbox, host callback adapter, or dynamic plugin/discovery boundary. The
goal is not to ban event-driven design; it is to avoid turning early orchestration convenience into a permanent
dependency magnet.

## Change Review Checklist

These points are worth re-checking when changing ownership, communication, or runtime boundaries:

- Keep the Rust and Kotlin build roots separate. Do not make Cargo own the Kotlin extension, and do not make Gradle own
  the Rust workspace.
- Plugin mode is the production target and drives the realtime constraints. Detailed Rust-side rules live in
  [Rust workspace architecture](../rust/architecture.md) and [Plugin flow](plugin-flow.md).
- Plugin parameter synchronization is intentionally bidirectional. Before changing it, re-check
  [Runtime lifecycle](runtime-lifecycle.md) and preserve the current distinction between host-origin and GUI-origin
  updates.
- Prefer typed peer boundaries wired at bootstrap over adding more cases to a runtime-wide event bus. If a bootstrap
  section starts looking like a hidden orchestrator, split the peer protocol instead of centralizing more behavior.
- [Runtime lifecycle](runtime-lifecycle.md) is the authoritative data-flow/thread-boundary diagram.

## Detailed Flow Notes

- [Runtime lifecycle](runtime-lifecycle.md) documents bootstrap wiring, ownership boundaries, and the main data flows
  after startup across plugin, desktop, and WASM mode.
- [Native MIDI flow](native-midi-flow.md) documents the desktop MIDI service, BPM worker, output thread, and the
  closure-command boundary used by `MidiService::execute()`.
- [Plugin flow](plugin-flow.md) documents host buffer processing, realtime handoff, background BPM work, and plugin
  tempo feedback.
- [Algorithm archaeology](algorithm-archaeology.md) documents the original interval/uncertainty idea, why the histogram
  exists, and why visualization became part of the development loop.
