# Native MIDI Flow

This document describes the desktop/native MIDI path. It is intentionally scoped to communication boundaries: where data
comes from, which thread owns which state, and why some boundaries use explicit messages while others use closures.

## Thread Boundaries

```text
TUI service
  -> MidiService::execute(closure)
  -> MIDI Service thread
      -> MidiIn
      -> midir input callback
          -> timed MIDI message
          -> TUI display callback
          -> WorkerEvent, when the BPM worker can use it
              -> BPM worker thread
                  -> BPMDetection
                  -> BPMDetectionReceiver
                  -> MidiOutputCommand, for optional tempo feedback
                      -> MIDI output thread
```

## Main Terms

- Timed MIDI message: a parsed runtime MIDI message plus timestamp. Native mode keeps these for the TUI MIDI display and
  SysEx parsing.
- Worker event: a filtered command sent to the BPM worker. The worker only receives messages it can act on, such as
  note-on events, config changes, or play/stop transport commands.
- Note-on event: the core BPM input observation. It is extracted from a timed MIDI message before entering
  `BPMDetection`.
- MIDI output command: a side effect for the native MIDI output thread, such as play, stop, or tempo feedback SysEx.

## Desktop Shell Archeology

The native desktop entry point started as a TUI because that looked like the quickest way to build the first experiment:
select a MIDI controller, show log-like feedback, and drive the BPM detector. In practice the TUI became its own source
of complexity. The current desktop app mostly keeps the TUI around for controller selection and the event loop, then
launches the egui GUI for the actual visualization and parameter workflow.

This means the TUI/event-bus shape should not be treated as final architecture. It is an artifact of the first working
proof of concept. The broad `tui::Event` and `Action` types encode terminal input, MIDI device discovery, MIDI messages,
screen commands, config updates, GUI launch commands, and service actions. That made early wiring possible, but it also
means unrelated components can become coupled through one large message surface.

The long-term direction is probably a native full-GUI desktop mode:

- controller selection moves into egui;
- the desktop event loop is rebuilt around the GUI/runtime actually being used;
- MIDI service operations keep explicit ownership boundaries instead of flowing through a catch-all TUI bus;
- async is used only where cooperative scheduling is needed, not as the default shape for a fixed set of background
  workers.

Until that refactor happens, code in `crates/tui` should be read as desktop shell scaffolding. It can still be useful and
working, but it is not the architectural model the rest of the project should copy.

## MIDI Service Closure Boundary

`bpm_detection_midi::MidiService` owns a dedicated service thread. Callers do not send a large public enum of every
possible service operation. Instead, they call `MidiService::execute()` with a closure.

That closure runs on the MIDI service thread with access to:

- `MidiIn`, which owns MIDI input setup and the worker command sender.
- the current `Option<MidiInputConnection<()>>`, whose lifetime controls whether the selected input is still listening.

This is deliberate. The caller can express an operation using the most suitable local data and return path, while
`MidiService` keeps the thread-affinity and synchronization ceremony internal. If the closure needs to report back, it
can use the caller's chosen mechanism, such as returning a `Result`, sending an app event, or mutating the input
connection holder provided by the service thread.

The tradeoff is that callers must move only thread-safe state into the closure and must expect execution to fail if the
service thread is gone.

## Explicit Worker Messages

The BPM worker uses explicit `WorkerEvent` values instead of arbitrary closures. This boundary is narrower:

- note-on observations should enter the core model;
- static config changes should rebuild detection buffers after the debounce delay;
- dynamic config changes should update scoring weights and trigger a delayed evaluation;
- play/stop commands should be forwarded to the MIDI output thread.

An enum is reasonable here because the worker protocol is small and part of the runtime design. It also prevents high
volume input traffic, such as MIDI Timing Clock, from waking the BPM worker when the worker has no use for it.

## Output Ownership

The native virtual MIDI output is owned by the MIDI output thread. The BPM worker does not call output methods directly.
Instead it sends `MidiOutputCommand` values.

This keeps clock ticks, play/stop, and optional tempo SysEx serialized through one owner. Tempo updates are coalesced
while draining output commands: when several tempo values are queued, only the newest value is emitted.

## Config Timing

Static and dynamic BPM config changes are both debounce points, but they mean different things:

- Static BPM config changes reshape the detection model and can rebuild buffers or precomputed data.
- Dynamic BPM config changes alter scoring while reusing the existing detection buffers.

The BPM worker delays evaluation briefly so related config changes can be applied together. Static config is applied at
that debounce boundary, then the worker recomputes BPM once.

## Validation Notes

These names and boundaries are current working vocabulary, not final doctrine:

- `MidiIn` is more than raw input: it also starts the BPM worker and forwards play/stop/config commands.
- `MidiService::execute()` is flexible, but the caller still needs to reason carefully about what it moves into the
  closure and how results should get back to the caller.
- The TUI event bus is early desktop-shell scaffolding. If behavior feels coupled through a large `Event` or `Action`
  enum, treat that as a refactor candidate rather than a design rule.
- The native MIDI clock path is desktop/experimental support. The plugin production path uses a controller bridge
  instead of acting as a MIDI clock provider.
