# MIDI BPM Detection

MIDI BPM Detection estimates tempo from incoming MIDI note-on events while you play. The goal is to let a musician record
freely, infer the tempo from the performance in realtime, and feed that tempo back to the host DAW so the recording can
fit a loop with less manual adjustment.

The detector compares intervals between recent notes, scores likely beat durations, and exposes both a single estimated
BPM and a histogram that shows competing tempo candidates. The histogram is important: it makes the guess inspectable
instead of hiding the model behind one number.

The screenshot below shows the plugin/demo UI with detection parameters and the realtime histogram. The strongest peak is
the current most likely BPM.

<a href="https://valsteen.github.io/midi_bpm_detection/"><img src="screenshot.png" alt="screenshot"></a>

Try the browser demo: https://valsteen.github.io/midi_bpm_detection/

## Project Shape

This is an experimental Rust project with three runtime modes:

- `plugin`: the CLAP/VST3 target intended to run inside a DAW. This is the production constraint.
- `desktop`: a native GUI app used for local iteration and native MIDI experiments.
- `wasm`: a browser demo that makes the detector easy to try and share.

The project is still a work in progress. Tempo detection depends on play style and parameter tuning, and the host tempo
feedback path is currently shaped around Bitwig integration.

The core BPM evaluation lives in
[crates/bpm_detection_core/src/bpm_detection.rs](crates/bpm_detection_core/src/bpm_detection.rs).

## Documentation

- [Architecture](docs/architecture.md): crate map, runtime modes, and architecture boundaries.
- [Runtime lifecycle](docs/runtime-lifecycle.md): bootstrap wiring and data flows between plugin, desktop, WASM, GUI, and
  BPM detection components.
- [Plugin flow](docs/plugin-flow.md): host buffer processing, realtime handoff, background work, and tempo feedback.
- [Native MIDI flow](docs/native-midi-flow.md): desktop MIDI service, controller boundary, worker messages, and native
  MIDI output ownership.
- [Algorithm archaeology](docs/algorithm-archaeology.md): the original tempo-detection idea and why the histogram exists.
- [Development commands](docs/development.md): setup, formatting, checking, plugin bundling, and WASM demo commands.

## Building And Using The CLAP/VST3 Plugin

Bundle the plugin with:

```shell
cargo xtask bundle midi-bpm-detector-plugin --release
```

The plugin artifacts are written under `target/bundled` as `midi-bpm-detector-plugin.clap` and
`midi-bpm-detector-plugin.vst3`.

To control the host DAW tempo, the plugin needs a companion controller integration. The current Bitwig controller script
is here:

https://github.com/valsteen/bitwig-beat-detection-controller

Load the controller script, add the CLAP plugin to the MIDI track, and select the plugin. The controller should detect it
and set the plugin's `DAW Port` parameter for local TCP communication. Then enable `Send tempo`.
