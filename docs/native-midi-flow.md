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
          -> BpmWorkerCommand, when the BPM worker can use it
              -> BPM worker thread
                  -> BPMDetection
                  -> BPMDetectionReceiver
                  -> MidiOutputCommand, for optional tempo feedback
                      -> MIDI output thread
```

## Main Terms

- Timed MIDI message: a parsed runtime MIDI message plus timestamp. Native mode keeps these for the TUI MIDI display and
  SysEx parsing.
- BPM worker command: a filtered command sent to the BPM worker. The worker only receives messages it can act on, such as
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
- producers and consumers are connected during bootstrap through narrow typed protocols, then communicate directly
  through those protocols;
- async is used only where cooperative scheduling is needed, not as the default shape for a fixed set of background
  workers.

Until that refactor happens, code in `crates/tui` should be read as desktop shell scaffolding. It can still be useful and
working, but it is not the architectural model the rest of the project should copy.

## Current Desktop Startup

The native desktop binary currently splits startup into two typed phases:

1. `PreparedTui` owns the configuration, action channel, `GuiRemote`, and `AppBuilder`.
2. `RunningTui` owns the Tokio runtime, the one-shot "start the GUI now" receiver, and the `AppBuilder`.

This shape is clever in a useful way: after preparation, the only operation left is to spawn the TUI runtime; after the
runtime is spawned, the only operation left on the main thread is waiting for the GUI start signal and running egui.
Those types encode lifecycle order instead of relying only on comments.

The runtime split exists because the current desktop app starts in the terminal:

```text
main thread
  -> read/update TUIConfig
  -> create GuiRemote + AppBuilder
  -> spawn Tokio runtime for the TUI shell
  -> block until Action::ShowGUI asks for egui
  -> run egui on the main thread

Tokio runtime
  -> run_tui()
      -> create TUI Event channel
      -> install signal handling
      -> install GUI exit callback
      -> start MIDI service wrapper
      -> start Ratatui event/render loops
      -> dispatch Event values into Action values
      -> dispatch Actions to TUI components and services
```

The GUI exit callback is part of that lifecycle handoff. When egui exits, it sends `Action::Quit` into the TUI loop and
then waits until the Tokio side reports that it has exited. This keeps terminal cleanup and MIDI service teardown under
the TUI runtime, but it also couples GUI shutdown to the terminal shell.

When the TUI is retired, the useful invariant is not "start Ratatui before egui". The useful invariant is that desktop
bootstrap should make ownership obvious:

- one owner starts the native MIDI service;
- one owner starts the GUI on the platform-required thread;
- closing the GUI has one clear path for stopping MIDI input, background workers, and config persistence;
- lifecycle order remains encoded in types or a small bootstrap object instead of being reconstructed from comments.

## TUI Port/Delete Checklist

The pieces to port into a GUI-native desktop mode are small:

- native MIDI input discovery and selection;
- hotplug refresh notifications;
- stable selection behavior when the device list changes;
- selected-device listening, where replacing the held `MidiInputConnection` stops the old listener;
- parameter/config propagation from the GUI into `MidiService`;
- optional desktop-only controls for native MIDI playback, MIDI clock, and tempo feedback if those experiments remain
  useful.

The pieces that should disappear with Ratatui unless they prove otherwise are broader:

- terminal drawing and alternate-screen lifecycle;
- TUI screen switching;
- TUI keybinding dispatch as the way the GUI talks back to the shell;
- `tui::Event` and `Action` as a runtime-wide bus;
- generic component/service dispatch over trait objects for the desktop GUI path.

If a replacement still needs events, they should be local to the thing that owns them. For example, a GUI device selector
may have a small model/update API, and the native MIDI service may keep closure commands or a narrow command enum. The
goal is not to avoid events; it is to avoid rebuilding the same global bus under an egui name.

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

This is intentionally not a direct translation of `Action`. For example, `Action::Down` and `Action::Switch` are TUI
interaction details and should disappear. `select_midi_input(port)` is a desktop capability and should remain.

The current desktop runtime uses a pending command runtime during bootstrap. The command sender exists before
`DesktopController` exists, but the command worker is only started once the controller has been fully constructed. This
handles the awkward native lifecycle without an `Option<DesktopController>` command target: macOS hotplug callbacks can
be registered before other MIDI initialization, those callbacks can enqueue work if they fire early, and the queued work
runs only after the real controller is installed.

The desktop MIDI input list refresh is platform-dependent. macOS registers CoreMIDI hotplug notifications and refreshes
the list automatically, so the GUI should not show a manual refresh button there. Other native platforms currently rely
on manual refresh because no equivalent hotplug callback is wired yet; the refresh still works by asking `midir` for the
current input ports again.

### Placement Options

There are three plausible places for this boundary:

1. Keep it temporarily inside `crates/tui` while deleting Ratatui-specific code around it.
2. Rename or replace `crates/tui` with a desktop/native app crate once the terminal UI is gone.
3. Add a new desktop integration crate and leave `crates/tui` as a shrinking compatibility shell until it can be removed.

The likely direction is option 2 or 3. Option 1 is useful only as a short migration step. The final shape should make the
dependency purpose obvious from the crate name: this code is the native desktop runtime, not a terminal UI.

### First Extraction Candidate

The first useful extraction is not a widget. It is the non-visual model behind MIDI input selection:

- current device list;
- current selected port;
- stable selection update when the device list changes;
- command to select a port and replace the active `MidiInputConnection`;
- callback/signal for a GUI widget to refresh when the device list changes.

Once that exists, egui can render it as a combo box or selector without owning MIDI service synchronization itself. The
existing `SelectDevice` component is the reference behavior to preserve, but its Ratatui drawing and `Action::Up` /
`Action::Down` handling are not part of the model.

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
- The TUI event bus is early desktop-shell scaffolding. If behavior feels coupled through a large `Event` or `Action`
  enum, treat that as a refactor candidate rather than a design rule.
- The native MIDI clock path is desktop/experimental support. The plugin production path uses a controller bridge
  instead of acting as a MIDI clock provider.
