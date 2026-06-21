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
- Worker event: a message sent to a background BPM worker. It is already filtered to something the worker can act on,
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

## Validation Notes

These points are worth validating before writing deeper runtime diagrams:

- `bpm_detection_core` now owns the algorithm/config/core-note surface, while `bpm_detection_midi` owns native MIDI
  service integration. If core grows again, keep checking whether new code belongs to the algorithm model, a
  MIDI-protocol adapter, or a runtime mode.
- `GuiRemote` is the shared UI update bridge, not just a plugin helper. It is used as the boundary between BPM producers
  and egui rendering.
- Plugin mode is the production target and drives the realtime constraints. Desktop and WASM preserve the same model but
  can use less restrictive runtime mechanisms.
- The most useful next diagram is probably a data-flow/thread-boundary diagram, not a sequence diagram. Sequence diagrams
  will be useful later for specific flows such as "plugin MIDI note received" or "GUI parameter change propagates".

## Detailed Flow Notes

- [Native MIDI flow](native-midi-flow.md) documents the desktop MIDI service, BPM worker, output thread, and the
  closure-command boundary used by `MidiService::execute()`.
