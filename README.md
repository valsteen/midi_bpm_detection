# Midi BPM detection

WASM Demo: https://valsteen.github.io/midi_bpm_detection/

This project performs a real-time analysis of midi notes and determines the BPM. Ultimately it aims to be a Clap/VST3
plugin ( located in [crates/midi-bpm-detector-plugin](crates/midi-bpm-detector-plugin) ) but as a proof of concept it started as a standalone program
( [crates/tui](crates/tui) ). There is also a wasm version to easily showcase what it does, linked above.

## State of the project

This is in a very early works-on-my-machine state, it even relies on forks of several dependencies that I will have to
re-evaluate and cleanup.

In the meantime curious developers may simply have a look at the model, the core of the BPM evaluation can be found in
[midi/bpm_detection.rs](crates/midi/src/bpm_detection.rs).