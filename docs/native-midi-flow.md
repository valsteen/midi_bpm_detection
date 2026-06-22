# Native MIDI Flow

This document describes the desktop/native MIDI path. It is intentionally scoped to communication boundaries: where data
comes from, which thread owns which state, and why some boundaries use explicit messages while others use closures.

## Thread Boundaries

```text
desktop bootstrap
  -> create GuiRemote
  -> create pending DesktopController runtime
  -> create MidiService
      -> MIDI Service thread
          -> MidiIn
          -> midir input callback
              -> timed MIDI message
              -> BpmWorkerCommand, when the BPM worker can use it
                  -> BPM worker thread
                      -> BPMDetection
                      -> BPMDetectionReceiver
                      -> MidiOutputCommand, for optional tempo feedback
                          -> MIDI output thread
  -> create DesktopController
  -> start DesktopController command worker
  -> start egui
```

## Main Terms

- Timed MIDI message: a parsed runtime MIDI message plus timestamp. Native mode uses these for SysEx parsing and for
  forwarding useful observations into the BPM worker.
- BPM worker command: a filtered command sent to the BPM worker. The worker only receives messages it can act on, such as
  note-on events, config changes, or play/stop transport commands.
- Note-on event: the core BPM input observation. It is extracted from a timed MIDI message before entering
  `BPMDetection`.
- MIDI output command: a side effect for the native MIDI output thread, such as play, stop, or tempo feedback SysEx.

## Desktop Shell Archeology

The native desktop entry point started as a TUI because that looked like the quickest way to build the first experiment:
select a MIDI controller, show log-like feedback, and drive the BPM detector. In practice the TUI became its own source
of complexity, and the project now uses `crates/desktop` as the native app path.

This means the TUI/event-bus shape should not be treated as final architecture. It is an artifact of the first working
proof of concept. The broad `tui::Event` and `Action` types encode terminal input, MIDI device discovery, MIDI messages,
screen commands, config updates, GUI launch commands, and service actions. That made early wiring possible, but it also
means unrelated components can become coupled through one large message surface.

The native GUI path keeps the useful pieces and drops the terminal shell:

- controller selection moved into egui;
- MIDI service operations keep explicit ownership boundaries instead of flowing through a catch-all event bus;
- producers and consumers are connected during bootstrap through narrow typed protocols, then communicate directly
  through those protocols;
- async is avoided for the fixed set of native background workers unless cooperative scheduling is actually needed.

## Current Desktop Startup

The native desktop binary starts egui directly and keeps MIDI work behind explicit service/controller boundaries:

- `main` loads config and creates `GuiRemote`.
- `PendingDesktopControllerRuntime` creates a command sender before `DesktopController` exists.
- `MidiService` is created during bootstrap so native MIDI setup and macOS hotplug registration happen before the
  desktop controller wraps the service.
- `DesktopController` owns device selection, selected input lifetime, and config propagation into `MidiService`.
- The command worker starts only after the controller exists, so queued callbacks never operate on an unset controller.
- `AppBuilderShell` receives `DesktopBaseConfig` only after the controller/runtime are ready.

## Proposed Desktop Controller Boundary

The future desktop GUI needs an integration layer above `gui` and `bpm_detection_midi`.

That layer should depend on both crates, but neither shared crate should depend back on it:

- `gui` should stay reusable by plugin and WASM mode, so it should not learn about `midir`, `MidiInputPort`, native
  hotplug callbacks, or desktop-only playback/clock controls.
- `bpm_detection_midi` should stay UI-free. It should own native MIDI service mechanics, not egui widgets or window
  lifecycle.
- The desktop controller should own the relationship between the two: native device selection, selected input lifetime,
  MIDI display/debug state if still useful, config propagation, and desktop-only MIDI side effects.

The controller should expose capabilities that map to user intent rather than TUI actions:

```text
DesktopController
  -> list MIDI inputs
  -> select MIDI input
  -> observe MIDI input list changes
  -> apply static BPM config
  -> apply dynamic BPM config
  -> toggle native playback / MIDI clock / tempo feedback, if kept
  -> stop native services on shutdown
```

This is intentionally not a direct translation of the old TUI `Action` enum. For example, keyboard navigation actions
were terminal interaction details and disappeared. `select_midi_input(port)` is a desktop capability and remains.

The current desktop runtime uses a pending command runtime during bootstrap. The command sender exists before
`DesktopController` exists, but the command worker is only started once the controller has been fully constructed. This
handles the awkward native lifecycle without an `Option<DesktopController>` command target: macOS hotplug callbacks can
be registered before other MIDI initialization, those callbacks can enqueue work if they fire early, and the queued work
runs only after the real controller is installed.

The desktop MIDI input list refresh is platform-dependent. macOS registers CoreMIDI hotplug notifications and refreshes
the list automatically, so the GUI should not show a manual refresh button there. Other native platforms currently rely
on manual refresh because no equivalent hotplug callback is wired yet; the refresh still works by asking `midir` for the
current input ports again.

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

The BPM worker uses explicit `BpmWorkerCommand` values instead of arbitrary closures. This boundary is narrower:

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

MIDI clock enablement is currently a shared atomic flag read by the output thread. That avoids blocking a GUI caller on
the output owner, but it also means the output thread uses a short idle timeout while the clock is disabled. A better
future shape is likely an event plus state pair: writers update cheap shared state, then wake the output owner so it can
react without polling. Any change here should keep the clock tick path owned by one thread and avoid introducing native
MIDI dependencies into `gui`.

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
- The old TUI event bus was early desktop-shell scaffolding. If behavior starts coupling through a large runtime-wide
  event enum again, treat that as a refactor candidate rather than a design rule.
- The native MIDI clock path is desktop/experimental support. The plugin production path uses a controller bridge
  instead of acting as a MIDI clock provider.
