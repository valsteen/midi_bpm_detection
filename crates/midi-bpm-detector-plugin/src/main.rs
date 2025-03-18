use cpal::{HostId, traits::HostTrait};
use errors::initialize_logging;
use midi_bpm_detector_plugin::MidiBpmDetector;
use midir::os::unix::{VirtualInput, VirtualOutput};
use nih_plug::wrapper::standalone::{
    backend::CpalMidir,
    config::{BackendType, WrapperConfig},
    wrapper::Wrapper,
};

fn main() {
    initialize_logging().unwrap();

    let _tui_output = midir::MidiOutput::new("TUI").unwrap().create_virtual("TUI").unwrap();
    let bpm_output = midir::MidiOutput::new("TUI").unwrap().create_virtual("BPM").unwrap();
    let _midi_input = midir::MidiInput::new("TUI")
        .unwrap()
        .create_virtual("TUI", |_time, midi_message, midi_output| midi_output.send(midi_message).unwrap(), bpm_output)
        .unwrap();
    let _out = cpal::host_from_id(HostId::CoreAudio).unwrap().output_devices().unwrap().next().unwrap();

    let config = WrapperConfig {
        backend: BackendType::CoreAudio,
        audio_layout: None,
        sample_rate: 48000.0,
        period_size: 512,
        input_device: None,
        output_device: None,
        midi_input: Some("BPM".to_string()),
        midi_output: None,
        connect_jack_inputs: None,
        connect_jack_midi_input: None,
        connect_jack_midi_output: None,
        dpi_scale: 1.0,
        tempo: 120.0,
        timesig_num: 4,
        timesig_denom: 4,
    };

    let backend = CpalMidir::new::<MidiBpmDetector>(config.clone(), HostId::CoreAudio).unwrap();
    let wrapper = Wrapper::<MidiBpmDetector, _>::new(backend, config).unwrap();

    wrapper.run().unwrap();
}
