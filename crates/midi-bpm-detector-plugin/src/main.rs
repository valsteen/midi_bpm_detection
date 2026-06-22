use cpal::{HostId, traits::HostTrait};
use errors::initialize_logging;
use midi_bpm_detector_plugin::MidiBpmDetector;
use midir::os::unix::{VirtualInput, VirtualOutput};
use nih_plug::wrapper::standalone::nih_export_standalone_with_args;

fn main() {
    initialize_logging().unwrap();

    let _tui_output = midir::MidiOutput::new("TUI").unwrap().create_virtual("TUI").unwrap();
    let bpm_output = midir::MidiOutput::new("TUI").unwrap().create_virtual("BPM").unwrap();
    let _midi_input = midir::MidiInput::new("TUI")
        .unwrap()
        .create_virtual("TUI", |_time, event, midi_output| midi_output.send(event).unwrap(), bpm_output)
        .unwrap();
    let _out = cpal::host_from_id(HostId::CoreAudio).unwrap().output_devices().unwrap().next().unwrap();

    let standalone_started = nih_export_standalone_with_args::<MidiBpmDetector, _>([
        "midi-bpm-detector-plugin".to_string(),
        "--backend".to_string(),
        "core-audio".to_string(),
        "--midi-input".to_string(),
        "BPM".to_string(),
    ]);

    if !standalone_started {
        std::process::exit(1);
    }
}
