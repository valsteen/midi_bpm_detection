# Native MIDI Flow

This document describes the desktop application's native MIDI boundaries: GUI-owned values, controller commands,
service-thread ownership, BPM work, and virtual MIDI output.

## Runtime Shape

```text
desktop bootstrap
  -> create BPMDetectionGUI, BpmDisplayPublisher, and GuiContextHandle
  -> create PendingDesktopControllerRuntime and its command queue
  -> create MidiService
      -> MIDI service thread owns MidiIn and the active input connection
          -> midir callback converts useful input to BpmWorkerCommand
              -> BPM worker thread owns BPMDetection
                  -> BpmDisplayPublisher
                  -> MidiOutputCommand
                      -> MIDI output thread owns the virtual output
  -> create DesktopController and start its command worker
  -> create DesktopApp and start eframe
```

The desktop entrypoint wires these owners together. The shared `gui` crate has no native MIDI dependency, and
`bpm_detection_midi` has no egui dependency. `DesktopController` is the integration boundary between them.

## Desktop-Owned GUI State

`DesktopApp` owns four kinds of runtime-facing state:

- an in-memory `DesktopConfig` value;
- the `EditableSettings` proposal passed to `BPMDetectionGUI::show`;
- a GUI-local `DeviceSelection` snapshot and its `displayed_revision`;
- the concrete `DesktopControllerCommandQueue` used after an edit.

`DesktopApp::logic` calls `BPMDetectionGUI::prepare` and attempts a nonblocking controller lock. `DeviceSelection`
increments its revision when the published device list, displayed selection, or selected device changes. The app clones
the complete controller snapshot only when that revision differs from `displayed_revision`; otherwise it retains the
existing GUI value. A busy controller retains the previous snapshot in memory and temporarily replaces the device
controls with the GUI-local `controller_busy` indicator.

`DesktopApp::ui` renders device controls and the shared BPM settings, then calls `DesktopApp::commit` with the returned
`GuiChanges` and `DesktopChanges`. The commit updates the app's in-memory config mirror and invokes only the concrete
capability associated with the edit. Configuration persistence is a separate `DesktopConfig::save` operation and is not
part of this frame commit.

## Controller Command Boundary

`DesktopControllerCommandQueue` exposes five product commands:

- `apply_static_config(StaticBPMDetectionConfig)`;
- `apply_dynamic_config(DynamicBPMDetectionConfig)`;
- `set_send_tempo(bool)`;
- `refresh_devices(GuiContextHandle)`;
- `select_device_index(usize)`.

Its generic closure-bearing `send` method is private. The command worker receives those closures and runs them against
the single `DesktopController`. A weak queue handle supports native device-change callbacks without creating an ownership
cycle; when the strong queue is gone, the callback stops enqueueing work.

The controller uses `MidiService::execute` for operations that require the MIDI service thread. Selecting an input
replaces the service-owned `Option<MidiInputConnection<()>>`; dropping that value stops the previous listener. Static and
dynamic detector settings become explicit `BpmWorkerCommand` values through `MidiIn`.

Send-tempo changes use this focused path:

```text
DesktopApp::commit
    -> DesktopControllerCommandQueue::set_send_tempo
    -> DesktopController::set_send_tempo
    -> MidiService::set_send_tempo
    -> MidiOutputRuntimeState::set_send_tempo
```

The worker's detected-BPM publisher later reads that live value before queuing `MidiOutputCommand::Tempo`.

## Device Discovery and Selection

`DesktopController::refresh_devices` asks the service-owned `MidiIn` for current ports, sorts the resulting values, and
updates `DeviceSelection` while retaining the selected device when it is still present. A selection change connects the
new input first and publishes the new selection only after that connection succeeds.

On macOS, CoreMIDI hotplug notification holds a weak controller queue and requests `refresh_devices`; the command asks
`GuiContextHandle` for a repaint after refreshing. Other native platforms expose the manual refresh button in the desktop
UI.

## Value Configuration and Live Runtime State

`DesktopConfig` serializes `MidiServiceConfig` alongside BPM settings. `MidiServiceConfig` contains detached values:

- `device_name: String`;
- `send_tempo: bool`;
- `enable_midi_clock: bool`.

Live atomic identity is private to `bpm_detection_midi`. `MidiService::new` copies the two booleans into the crate-private
`MidiOutputRuntimeState` and retains a clone. The BPM publisher reads the shared send-tempo atomic, and the MIDI-output
worker reads the shared clock-enable atomic. The serialized configuration never contains or exposes either atomic.

Two other atomics have similarly narrow runtime owners:

- `MidiIn::start_timestamp` is shared with the active midir callback so native timestamps can be converted to elapsed
  time from the current listener start;
- native worker bootstrap creates `clock_interval_microseconds`, then shares it only with the detected-BPM publisher and
  MIDI-output thread.

`enable_midi_clock` is initialized from configuration and observed by the output worker. `send_tempo` also has the live
setter shown above.

## MIDI Input and BPM Worker

The midir callback parses each native message and establishes an elapsed timestamp. Tempo SysEx updates the display
publisher directly. Messages convertible to the narrow `BpmWorkerCommand` protocol enter the BPM worker; unrelated
high-volume input such as MIDI Timing Clock does not wake that worker.

The worker protocol contains note-on observations, static and dynamic detector settings, and play/stop actions. Static
configuration is retained until the evaluation debounce boundary and then rebuilds detector buffers. Dynamic
configuration replaces the current scoring values and schedules evaluation without rebuilding the model shape.

## Output Ownership

The MIDI-output thread exclusively owns the virtual output. The BPM worker communicates with it through
`MidiOutputCommand` values for play, stop, and detected tempo. While the clock-emitter loop is active, draining coalesces
multiple queued tempo commands so the newest value is emitted. With clock emission disabled, the output thread dispatches
each command received through its timed wait.

Detected BPM also updates `clock_interval_microseconds`. When MIDI clock is enabled, the output owner reads that interval
and emits ticks. When clock is disabled, it waits with a short timeout so it can process output commands and observe a
later atomic enablement change. Optional tempo feedback is encoded as `TEMPO|...` SysEx by the same output owner.
