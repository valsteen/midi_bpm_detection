# Architecture

This document is a first-pass map of the project. It is intentionally high level: it should help a human contributor or
AI agent understand the main boundaries before following the detailed runtime data flows.

## Purpose

The project estimates the tempo of incoming MIDI notes in realtime. The core algorithm compares intervals between recent
notes, scores likely beat durations, and exposes the most likely BPM plus histogram data for visualization.

The same BPM detection model is used in three operating modes:

- `plugin`: a CLAP/VST3 plugin intended to run inside a DAW. This is the production target.
- `desktop`: a native TUI/GUI development app.
- `wasm`: a browser demo using the shared egui UI.

The main architectural goal is to keep these modes from importing unnecessary dependencies from each other. Each mode
owns its host/runtime integration, while shared crates carry the algorithm, configuration shapes, reusable GUI, and small
cross-platform abstractions.

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

## Crate Map

### Core Domain

- `crates/bpm_detection_core`
  - Owns the BPM detection algorithm.
  - Defines the in-house note event shape consumed by the algorithm, static/dynamic BPM detection config, and BPM
    conversion helpers.
  - Does not depend on native MIDI runtimes or a MIDI protocol parser.
  - Exposes `BPMDetectionReceiver`, the callback boundary used to publish detected BPM and histogram data.

- `crates/parameter`
  - Defines generic parameter metadata and value conversion helpers.
  - Keeps parameter description reusable across GUI, plugin, and core config code.

### Shared Infrastructure

- `crates/sync`
  - Provides synchronization aliases/wrappers that differ by target.
  - Keeps platform-specific lock/atomic choices out of higher-level crates.

- `crates/errors`
  - Centralizes error reporting, logging, panic handling, and tracing helpers.

- `crates/build`
  - Provides build metadata and project directories shared by multiple binaries/crates.

### Shared GUI

- `crates/gui`
  - Owns the reusable egui UI for parameters, BPM legend, and histogram rendering.
  - Defines `GuiRemote`, the cross-thread/task bridge used to push BPM/histogram updates into the UI.
  - Does not own a specific runtime mode; plugin, desktop, and WASM provide the surrounding application/runtime.

### Runtime Modes

- `crates/midi-bpm-detector-plugin`
  - CLAP/VST3 integration via `nih-plug`.
  - Receives MIDI in the plugin `process` callback.
  - Parses host MIDI bytes at the plugin boundary and maps note-on events into the core note type.
  - Uses a fixed ring buffer and host background tasks so the realtime callback avoids expensive work.
  - Owns DAW/plugin parameter integration and optional tempo feedback to the Bitwig controller socket.

- `crates/tui`
  - Native development app with TUI screens and optional GUI.
  - Uses Tokio and native worker threads around `bpm_detection_midi::MidiService`.
  - Can perform blocking MIDI setup/work outside the UI event loop.

- `crates/wasm`
  - Browser demo wrapper.
  - Uses Trunk, wasm-bindgen, browser MIDI/keyboard input, and the shared egui UI.
  - Uses async browser tasks and bounded channels instead of native threads.

- `crates/midi-reset`
  - Small macOS utility for restarting CoreMIDI.
  - Kept separate from the main operating modes.

- `crates/bpm_detection_midi`
  - Native MIDI runtime used by the desktop mode.
  - Owns MIDI device discovery/input, virtual MIDI output, SysEx control messages, playback clock emission, and the
    worker threads around `BPMDetection`.
  - Owns full MIDI message display types for the TUI device view.
  - Kept out of plugin and WASM builds so those modes do not inherit native MIDI service dependencies.

- `crates/midi-bpm-detector-plugin/xtask`
  - Packaging helper for plugin bundles.

## Operating Mode Boundaries

The same conceptual pipeline appears in each mode:

```text
MIDI/key input -> runtime-specific parsing -> core note events -> BPMDetection -> histogram/BPM output
    -> UI and/or host integration
```

The important difference is where that pipeline is allowed to do work:

- In plugin mode, the audio/plugin callback is the constrained boundary. It should not block, allocate, or perform heavy
  BPM computation. It forwards compact events and schedules background work.
- In desktop mode, MIDI and BPM work can live in native worker threads. The TUI service layer bridges
  `bpm_detection_midi` back into the application event loop.
- In WASM mode, there are no native worker threads in the current design. Browser events and delayed recomputation are
  coordinated through async tasks and channels.

## Tempo Feedback

Tempo feedback has two historically different implementations:

- Desktop mode can act as a native virtual MIDI device through `bpm_detection_midi`. It can emit MIDI clock, play/stop,
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
read like clean configuration of the runtime graph, not like a second hidden orchestrator. If future runtime features
need pluggable components, they should follow the same shape: discover compatible producers and consumers, connect them,
then let that pair communicate through its own protocol.

Small explicit enums are still valid when the protocol is narrow and stable. `BpmWorkerCommand` is a good example: it
belongs to one worker boundary and does not try to describe the whole application.

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

## Realtime Constraints

The plugin crate is the production runtime and has the strictest execution constraints. The code reflects these
constraints:

- The plugin `process` callback parses incoming MIDI and pushes events with `try_push` into a fixed ring buffer.
- BPM computation runs from `nih-plug` background tasks, not directly from the audio callback.
- Cross-thread state crossing the callback boundary uses atomics, fixed buffers, or non-blocking handoff.
- GUI updates are indirect through `GuiRemote`; the UI can repaint from shared state without the audio callback owning UI
  work.

These constraints should be treated as design rules when changing plugin-mode code. If a change requires allocation,
blocking I/O, lock contention, or unbounded work, it belongs outside the realtime callback.

## Forked Plugin Dependencies

The plugin path currently uses forks of `nih-plug` and `egui-baseview`. This should be treated as a pragmatic extension
of upstream crates, not as a permanent divergence goal.

The `nih-plug` fork currently carries four kinds of changes.

First, the fork changes `TaskExecutor` from `Fn` to `FnMut`. This project hands NIH-plug a closure that owns the plugin's
background executor and calls `TaskExecutor::execute(&mut self, task)`. That executor mutates the BPM model, shared config,
GUI remote, DAW tempo connection, and ring-buffer receiver. A plain `Fn` executor cannot express that owned mutable state
without introducing extra interior-mutability ceremony around the whole executor.

Second, the fork keeps the plugin editor aligned with the shared GUI stack. `nih_plug_egui` is pointed at the local
`egui-baseview` fork, now on the same egui generation as the desktop and WASM GUI. The fork also removes NIH-plug-egui's
unconditional `request_repaint()` in the editor update loop because this project has explicit repaint paths through
`GuiRemote`; always repainting kept the editor active even while visually idle.

Third, the fork carries small compatibility fixes required by the newer egui generation, including the
`ResizableWindow` integration update and one explicit `f32` literal in NIH-plug-egui widget code.

Fourth, the fork restores exhaustive matching for `NoteEvent` and adds `NoteEvent::UnsupportedMidi` as an explicit raw
MIDI escape hatch. That variant is for compact MIDI-shaped messages NIH-plug does not model as first-class events, while
real SysEx messages starting with `0xf0` still use NIH-plug's `SysExMessage` path. This keeps the low-level passthrough
feature available for adjacent controller/host experiments without keeping the old SysEx-tempo hack in this project.

The abandoned experiment was to send tempo data through SysEx or SysEx-like MIDI routing so a DAW-side controller script
could read it. Host behavior around SysEx routing was too inconsistent and under-documented for that path to be a stable
feature. The standalone binary also uses NIH-plug's public standalone entry point now, so the fork no longer exposes
private standalone wrapper modules.

Forks should follow a forward-only policy:

- prefer moving to newer upstream dependency generations over patching stale transitive crates;
- keep fork diffs small, boring, and shaped like upstreamable compatibility work;
- pin commits in this repository so plugin builds are reproducible;
- periodically check whether upstream has caught up enough to drop the fork or reduce its diff.

The egui/wgpu update that removed `block v0.1.6` follows this policy. Instead of patching `block`, `metal`, or
`wgpu-hal` 25, the project moved to the upstream generation where Metal uses `block2`/`objc2`. The only fork updates
needed were compatibility bumps so the plugin editor and shared desktop/WASM GUI could stay on the same egui generation.

## Configuration Shape

The BPM model has two broad config groups:

- Static BPM detection config: changes that alter the detection model shape, such as BPM range, sample rate, and normal
  distribution settings.
- Dynamic BPM detection config: changes that affect scoring/evaluation weights while the model is running.

Each runtime mode adapts this shared config into its own host surface:

- plugin parameters in `midi-bpm-detector-plugin`
- TUI/GUI config in `tui`
- browser demo config in `wasm`

The current plugin code also distinguishes whether updates originate from the DAW parameter system or the GUI. That
origin matters because it determines which side is authoritative and which side needs to be refreshed.

### Plugin Parameter Synchronization

Plugin parameters have two interactive surfaces:

- the DAW/plugin-host parameter surface;
- the egui plugin editor.

Both surfaces need to stay in sync, but blindly reflecting every update in both directions can create feedback loops:
the DAW updates the plugin, the GUI mirrors the change, the GUI writes the value back through the plugin setter, and the
host treats that as another user edit.

The current plugin code handles this by tagging config tasks with `UpdateOrigin::Daw` or `UpdateOrigin::Gui`. The origin
decides which side is considered authoritative for that update and whether the other side must refresh its local config.
This is intentional architecture, but some surrounding code should be treated as archeology from that struggle:

- startup forces an initial parameter sync so saved DAW parameters populate the GUI config;
- `gui_must_update_config` tells the editor to reload config after DAW-originated changes;
- GUI-originated changes are delayed and batched before they reach the background task executor;
- static and dynamic config updates use similar but not identical refresh/recompute paths.

This area is a likely refactor target. The desired end state is a small, explicit parameter-sync protocol that documents
which surface owns an update, which side must refresh, and when BPM recomputation is required. That protocol should make
feedback-loop prevention obvious instead of depending on scattered flags and timing behavior.

## Validation Notes

These points are worth validating before writing deeper runtime diagrams:

- `bpm_detection_core` now owns the algorithm/config/core-note surface, while `bpm_detection_midi` owns native MIDI
  service integration. If core grows again, keep checking whether new code belongs to the algorithm model, a
  MIDI-protocol adapter, or a runtime mode.
- `GuiRemote` is the shared UI update bridge, not just a plugin helper. It is used as the boundary between BPM producers
  and egui rendering.
- Plugin mode is the production target and drives the realtime constraints. Desktop and WASM preserve the same model but
  can use less restrictive runtime mechanisms.
- Plugin parameter synchronization is intentionally bidirectional, but the current implementation may still contain
  workaround-shaped code from avoiding DAW/GUI feedback loops. Review this before documenting it as final design.
- Prefer typed peer boundaries wired at bootstrap over adding more cases to a runtime-wide event bus. If a bootstrap
  section starts looking like a hidden orchestrator, split the peer protocol instead of centralizing more behavior.
- The most useful next diagram is probably a data-flow/thread-boundary diagram, not a sequence diagram. Sequence diagrams
  will be useful later for specific flows such as "plugin MIDI note received" or "GUI parameter change propagates".

## Detailed Flow Notes

- [Native MIDI flow](native-midi-flow.md) documents the desktop MIDI service, BPM worker, output thread, and the
  closure-command boundary used by `MidiService::execute()`.
- [Plugin flow](plugin-flow.md) documents host buffer processing, realtime handoff, background BPM work, and plugin
  tempo feedback.
- [Algorithm archaeology](algorithm-archaeology.md) documents the original interval/uncertainty idea, why the histogram
  exists, and why visualization became part of the development loop.
