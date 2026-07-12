# Runtime Lifecycle

This document shows how the runtime pieces are connected and what crosses each boundary after startup. It complements
the Rust crate-level map in [Rust workspace architecture](../rust/architecture.md), the production plugin notes in
[plugin-flow.md](plugin-flow.md), and the native MIDI notes in [native-midi-flow.md](native-midi-flow.md).

The main design rule is that startup wires peers together, then those peers communicate through the narrow relationship
they actually own. Bootstrap knows the graph. The graph should not turn into a runtime-wide event bus.

## Boundary Vocabulary

- **Bootstrap/composition root:** the runtime entry point that creates components, hands out callbacks, and connects
  producers to consumers.
- **Realtime callback:** the host plugin `process()` callback. It can parse block-local facts and enqueue compact work,
  but it must not block or do heavy BPM computation.
- **Worker mailbox:** a small message protocol owned by one worker boundary. `Task` and `BpmWorkerCommand` are examples;
  neither is a general application event enum.
- **Service closure:** a command closure executed by the owning service thread. `MidiService::execute()` uses this shape
  so native MIDI thread ownership stays inside `bpm_detection_midi`.
- **Remote receiver:** a narrow receiver used by BPM producers to push display data. `GuiRemote` implements
  `BPMDetectionReceiver` and owns repaint requests plus shared GUI-facing state.
- **Latest-state GUI handoff:** a display update boundary where producers publish the newest BPM/histogram state through
  `GuiRemote`. It is not a durable queue. The GUI reads the latest shared state on its next frame, and contention paths
  prefer skipping/logging an update or frame over waiting for a blocking render lock.

## At-A-Glance Runtime Shape

```mermaid
flowchart LR
    subgraph plugin["Plugin runtime (production)"]
        direction TB
        host["DAW host / Bitwig"]
        process["MidiBpmDetector::process<br/>realtime callback"]
        plugin_params["MidiBpmDetectorParams<br/>host parameters"]
        ring["fixed ring buffer<br/>Event::TimedNoteOn / Event::DawBPM"]
        plugin_tasks["nih-plug background task<br/>TaskExecutor"]
        plugin_model["BPMDetection<br/>plugin-owned model"]
        plugin_gui["egui plugin editor<br/>GuiEditor + BaseConfig"]
        tempo_socket["localhost tempo controller<br/>optional Bitwig extension bridge"]

        host --> process
        host --> plugin_params
        plugin_gui --> plugin_params
        process --> ring --> plugin_tasks --> plugin_model
        plugin_tasks --> tempo_socket
    end

    subgraph desktop["Desktop runtime"]
        direction TB
        desktop_main["desktop main"]
        desktop_gui["shared egui app<br/>DesktopBaseConfig"]
        controller["DesktopController"]
        service["MidiService thread<br/>closure commands"]
        midi_in["MidiIn / midir callback"]
        native_worker["BPM worker thread<br/>BpmWorkerCommand"]
        native_output["MIDI output thread<br/>clock/play/stop/tempo SysEx"]

        desktop_main --> controller
        desktop_main --> desktop_gui
        desktop_gui --> controller
        controller --> service --> midi_in --> native_worker
        native_worker --> native_output
    end

    subgraph wasm["WASM demo runtime"]
        direction TB
        js["JavaScript / browser input"]
        wasm_gui["shared egui app<br/>BaseConfig"]
        wrapper["GuiRemoteWrapper"]
        wasm_queue["mpsc QueueItem channel"]
        wasm_task["browser async task"]
        wasm_model["BPMDetection<br/>WASM-owned model"]

        js --> wrapper --> wasm_queue --> wasm_task --> wasm_model
        wasm_gui --> wasm_queue
    end
```

Shared connections from the runtime row:

- `GuiRemote`: updated by plugin tasks, plugin editor setup, desktop bootstrap, native BPM worker, and the WASM async
  task.
- `bpm_detection_core`: used by each runtime-owned `BPMDetection` model and by the shared note/config types.
- `gui`: used by each runtime's GUI-facing config object and by `GuiRemote` for repaint/update state.

```mermaid
flowchart LR
    gui_remote["GuiRemote<br/>BPMDetectionReceiver"]
    gui["gui crate<br/>widgets + config accessors"]
    core["bpm_detection_core<br/>NoteOn + BPMDetection"]

    gui_remote --> gui
    gui --> core
```

The `gui` crate is deliberately shared and runtime-neutral. Plugin, desktop, and WASM mode each provide a config object
that implements the GUI-facing accessors. Those config objects are where runtime-specific side effects start.

## Plugin Bootstrap

```mermaid
sequenceDiagram
    participant Plugin as MidiBpmDetector::default()
    participant Config as PluginConfig
    participant Params as MidiBpmDetectorParams
    participant Ring as Event ring buffer
    participant Exec as TaskExecutor handoff
    participant Editor as GuiEditor handoff
    participant Host as nih-plug host wrapper

    Plugin->>Config: load built-in config
    Plugin->>Ring: split fixed Event buffer
    Plugin->>Params: create host params with change callbacks
    Params-->>Plugin: callbacks mark DeferredConfigUpdate at current sample
    Plugin->>Exec: create TaskExecutor with BPMDetection, shared config, ring receiver
    Plugin->>Editor: create GuiEditor with shared config and gui_remote_receiver
    Host->>Plugin: task_executor()
    Plugin-->>Host: move one-shot TaskExecutor closure
    Host->>Plugin: editor(async_executor)
    Plugin-->>Host: move one-shot GuiEditor into egui editor
    Host->>Plugin: initialize(buffer_config)
    Plugin->>Plugin: PluginTiming = Ready(sample_rate)
```

Who knows what:

- `MidiBpmDetector` owns the realtime callback state, host parameters, sample clock, deferred update markers, and the
  realtime-to-background ring producer.
- `TaskExecutor` owns the plugin BPM model, dynamic config snapshot, ring consumer, optional `GuiRemote`, and optional
  tempo-controller TCP connection.
- `GuiEditor` owns the plugin editor lifecycle and creates the GUI app when the editor opens.
- `GuiRemote` is passed by value across the runtime as a receiver/handle, but it only exposes the UI update surface.
- The `task_executor_handoff` and `gui_editor_handoff` fields are one-shot NIH-plug handoff slots, not general nullable
  runtime state.

## Plugin Notes

```mermaid
sequenceDiagram
    participant Host as DAW host process block
    participant Process as MidiBpmDetector::process()
    participant Ring as fixed Event ring
    participant Exec as TaskExecutor
    participant Model as BPMDetection
    participant Gui as GuiRemote
    participant Ui as BPMDetectionGUI frame
    participant Bridge as tempo controller socket

    Host->>Process: buffer + timed MIDI events + transport tempo
    Process->>Process: require PluginTiming::Ready(sample_rate)
    Process->>Ring: try_push(Event::DawBPM) when transport tempo exists
    loop each MIDI event in block
        Process->>Process: parse wmidi NoteOn
        Process->>Ring: try_push(Event::TimedNoteOn { absolute timestamp, note })
    end
    Process->>Exec: execute_background(Task::ProcessNotes) when data changed
    Exec->>Ring: drain events
    Exec->>Gui: receive_daw_bpm(bpm), if GUI remote exists
    Exec->>Model: receive_note_on(timed note)
    Model->>Model: append to note history buffer
    Exec->>Model: compute_bpm(dynamic config)
    Model->>Model: fill histogram buffer from note pairs
    Model-->>Exec: histogram data + detected BPM
    Exec->>Bridge: write length-prefixed f32 BPM, if enabled and connected
    Exec->>Gui: receive_bpm_histogram_data(histogram, bpm), if editor open
    Gui->>Gui: attempt best-effort histogram snapshot publication, store BPM atomically, request repaint
    Ui->>Gui: try_borrow latest histogram data on next frame
    Ui->>Ui: interpolate and draw histogram bars
```

Data that crosses the realtime boundary is intentionally small:

- `Event::TimedNoteOn(TimedNoteOn)` carries the timestamped core note observation.
- `Event::DawBPM(f32)` carries host transport tempo for display.
- `Task::ProcessNotes { force_evaluate_bpm_detection }` tells the background executor when the ring should be drained.

The realtime callback does not own GUI rendering, BPM computation, TCP writes, or host parameter reconciliation.

The histogram path is a latest-state handoff, not a note-by-note GUI queue:

- `BPMDetection::receive_note_on` mutates the model's note/history state.
- `BPMDetection::compute_bpm` clears and refills the model's histogram buffer from accumulated note pairs, returns
  `(histogram_data_points, detected_bpm)`, and prunes old notes after choosing a BPM.
- `GuiRemote::receive_bpm_histogram_data` copies the complete histogram into producer-owned reusable scratch, tries to
  swap it into the GUI-facing snapshot, stores the detected BPM in an atomic, and requests a repaint.
- `BPMDetectionGUI` tries to borrow the latest histogram data during a frame, interpolates from the previous drawn data,
  and renders the bars.

That boundary is useful because BPM producers do not wait for egui to draw. If the GUI is borrowing the snapshot when a
producer publishes, that visualization update is logged and dropped without retry while scalar BPM publication remains
independent. If the GUI cannot borrow the snapshot while drawing, that frame is skipped. This favors realtime/background
progress and responsive rendering over preserving every visual update.

Desktop and WASM use the same `GuiRemote` receiver shape after their worker/task computes a histogram.

## Plugin Parameter Changes From The Host

```mermaid
sequenceDiagram
    participant Host as DAW parameter surface
    participant Params as MidiBpmDetectorParams callbacks
    participant Marker as DeferredConfigUpdate
    participant Process as process()
    participant Exec as TaskExecutor
    participant Config as shared PluginConfig
    participant Gui as GuiEditor / BaseConfig
    participant Model as BPMDetection

    Host->>Params: set static/dynamic/gui parameter
    Params->>Marker: mark_changed_at_if_idle(current_sample)
    Process->>Marker: after 50 ms worth of samples
    Process->>Exec: Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Host)
    Process->>Exec: Task::GUIConfig(ParameterSyncOrigin::Host)
    Process->>Exec: Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Host)
    Exec->>Config: copy authoritative values from host params
    Exec->>Gui: gui_must_update_config = true
    Exec->>Model: update static config or dynamic config snapshot
    Exec->>Model: force ProcessNotes recompute for static/dynamic changes
    Gui->>Config: reload config on next editor update
```

The host parameter surface is authoritative for host-origin parameter sync. The GUI refreshes from shared config instead
of echoing the same edit back to the host as a new user action.

## Plugin Parameter Changes From The GUI

```mermaid
sequenceDiagram
    participant Editor as egui plugin editor
    participant Live as LiveConfig / BaseConfig
    participant Setter as nih-plug ParamSetter
    participant Config as shared PluginConfig
    participant Exec as TaskExecutor
    participant Model as BPMDetection
    participant GuiRemote as GuiRemote

    Editor->>Live: user edits GUI control
    Live->>Setter: begin/set/end matching host parameter
    Live->>Live: delay static, GUI/display, or dynamic change for 200 ms
    Live->>Config: write edited PluginConfig after delay
    Live->>Exec: Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Gui)
    Live->>Exec: Task::GUIConfig(ParameterSyncOrigin::Gui)
    Live->>Exec: Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Gui)
    Exec->>Config: read GUI-authored config
    Exec->>Model: apply static config or dynamic snapshot
    Exec->>Model: recompute on forced ProcessNotes for static/dynamic changes
    Exec->>GuiRemote: request repaint for GUI/display changes
```

The GUI-origin path intentionally writes host parameters through `ParamSetter`, because the DAW surface must reflect the
editor change. The GUI-origin parameter sync tasks then let the background BPM model consume the already-shared config
without treating the host callback as the source of truth.

The plugin code names the origin with `ParameterSyncOrigin::Host` or `ParameterSyncOrigin::Gui`; the consequences are
fixed:

| Origin | Authoritative surface | Coalescing window | GUI refresh | Host refresh | BPM recompute |
| --- | --- | --- | --- | --- | --- |
| Host/DAW | host parameters | 50 ms on the host sample clock | reload from shared config | already current | immediate worker task for static/dynamic changes |
| GUI | shared GUI-authored config | 200 ms on the editor wall clock | already current | `ParamSetter` write-through before the worker task | next realtime `process()` block for static/dynamic changes |

This table documents behavior, not optional capabilities. It is useful because the worker receives both host-origin and
GUI-origin config tasks. At origin-specific call sites, keep known facts direct: the host path uses the host coalescing
window, the GUI path uses the GUI coalescing window, static and dynamic changes mark BPM detection for re-evaluation, and
GUI/display changes repaint without forcing a BPM recompute.

## Plugin GUI Open And Close

```mermaid
sequenceDiagram
    participant Host as nih-plug editor host
    participant Editor as GuiEditor
    participant Gui as gui::create_gui
    participant RemoteCell as gui_remote_receiver
    participant Exec as TaskExecutor

    Host->>Editor: build(egui context, async executor)
    Editor->>Gui: create_gui(BaseConfig)
    Gui-->>Editor: GuiRemote + AppBuilder
    Editor->>RemoteCell: store(Some(GuiRemote))
    Editor->>Editor: force_evaluate_bpm_detection = true
    Exec->>RemoteCell: take new GuiRemote on next ProcessNotes
    Exec->>Exec: drop GuiRemote when editor_state is closed
```

This is why `GuiRemote` is not constructed directly inside the background executor. The GUI runtime owns the actual egui
context; the background task only picks up a remote handle after the editor has been built.

## Desktop Bootstrap

```mermaid
sequenceDiagram
    participant Main as desktop main
    participant Runtime as PendingDesktopControllerRuntime
    participant Gui as gui::create_gui_shell
    participant Service as MidiService
    participant MidiIn as MidiIn
    participant Worker as BPM worker
    participant Controller as DesktopController
    participant App as DesktopBaseConfig

    Main->>Runtime: create pending command runtime
    Main->>Runtime: get DesktopControllerCommandQueue
    Main->>Gui: create GuiRemote + AppBuilderShell
    Main->>Service: MidiService::new(config, GuiRemote, optional hotplug callback)
    Service->>MidiIn: start MIDI Service thread
    MidiIn->>Worker: spawn BPM worker and MIDI output thread
    Service-->>Main: ready command sender
    Main->>Controller: wrap MidiService
    Controller->>Service: refresh_devices()
    Main->>Runtime: start(controller)
    Main->>App: build DesktopBaseConfig with controller and callbacks
    Main->>Gui: start_gui(app_builder)
```

Desktop startup is the clearest example of "glue at startup, peers after that." `main` wires the GUI, controller,
command queue, service thread, worker, and output thread. After bootstrap:

- the GUI does not know `MidiIn` or `midir`;
- `bpm_detection_midi` does not know egui;
- the desktop controller is the one native bridge between those dependency surfaces;
- queued controller commands are closures targeted at `DesktopController`, not a desktop-wide action enum.

## Desktop MIDI Notes

```mermaid
sequenceDiagram
    participant Device as selected MIDI input
    participant MidiIn as midir callback / MidiIn
    participant Worker as BPM worker mailbox
    participant Model as BPMDetection
    participant Gui as GuiRemote
    participant Output as MIDI output thread

    Device->>MidiIn: raw MIDI bytes + native timestamp
    MidiIn->>MidiIn: convert timestamp to elapsed duration
    MidiIn->>Gui: receive_daw_bpm(bpm), for legacy tempo SysEx input
    MidiIn->>Worker: BpmWorkerCommand::TimedNoteOn, if message is note-on
    Worker->>Model: receive_note_on(timed note)
    Model->>Model: append to note history buffer
    Worker->>Model: compute_bpm(dynamic config)
    Model->>Model: fill histogram buffer from note pairs
    Model-->>Worker: histogram data + detected BPM
    Worker->>Output: MidiOutputCommand::Tempo(bpm), if send_tempo enabled
    Worker->>Gui: receive_bpm_histogram_data(histogram, bpm)
```

`BpmWorkerCommand` filters the native MIDI stream before it reaches the BPM worker. High-volume messages such as MIDI
Timing Clock do not wake the worker unless the worker owns a reason to act on them.

## Desktop User Actions

```mermaid
sequenceDiagram
    participant Gui as DesktopBaseConfig / egui controls
    participant Queue as DesktopControllerCommandQueue
    participant Controller as DesktopController
    participant Service as MidiService::execute()
    participant MidiIn as MidiIn
    participant Worker as BPM worker

    Gui->>Queue: select_device_index(index)
    Queue->>Controller: run queued closure
    Controller->>Service: execute closure with MidiIn and input holder
    Service->>MidiIn: listen(port)
    Service->>Service: replace input_connection holder

    Gui->>Queue: apply_static_config(config)
    Queue->>Controller: run queued closure
    Controller->>Service: execute(change_bpm_detection_config)
    MidiIn->>Worker: BpmWorkerCommand::StaticBPMDetectionConfig

    Gui->>Queue: apply_dynamic_config(config)
    Queue->>Controller: run queued closure
    Controller->>Service: execute(change_bpm_detection_config_live)
    MidiIn->>Worker: BpmWorkerCommand::DynamicBPMDetectionConfig
```

The GUI can display and request native operations, but it does not directly hold the MIDI input connection or worker
sender. The selected input lifetime is controlled by the service thread's `Option<MidiInputConnection<()>>`; replacing or
dropping that holder starts or stops listening.

## WASM Demo Flow

```mermaid
sequenceDiagram
    participant JS as JavaScript / browser input
    participant Wrapper as GuiRemoteWrapper
    participant Queue as mpsc QueueItem
    participant Task as wasm async task
    participant Model as BPMDetection
    participant Gui as GuiRemote

    JS->>Wrapper: event_in(channel, note, velocity, timestamp)
    Wrapper->>Queue: QueueItem::Note(TimedEvent<NoteOn>)
    Gui->>Queue: QueueItem::StaticParameters / DynamicParameters
    Task->>Task: debounce note/config updates
    Task->>Model: update_static_config or receive_note_on
    Task->>Model: compute_bpm(dynamic config)
    Task->>Gui: receive_bpm_histogram_data(histogram, bpm)
```

WASM follows the same conceptual pipeline, but browser constraints replace native threads with async tasks and bounded
channels. It is useful for demos and model/UI iteration, not the production constraint.
