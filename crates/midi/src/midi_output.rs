#![cfg(unix)]
use errors::MakeReportExt;
use log::{error, info};
use midir::{MidiOutputConnection, os::unix::VirtualOutput};
use wmidi::{Channel, ControlFunction, MidiMessage, U7};

use crate::midi_output_trait::{MIDI_CLOCK_MESSAGE, MIDI_PLAY_MESSAGE, MIDI_STOP_MESSAGE, MidiOutput};
use errors::{LogErrorWithExt, Result};

pub struct VirtualMidiOutput {
    virtual_output: MidiOutputConnection,
}

impl VirtualMidiOutput {
    pub fn new(device_name: &str) -> Result<Self> {
        let midi_output = midir::MidiOutput::new(device_name)?;
        let virtual_output = midi_output.create_virtual(device_name).report_msg("unable to create virtual output")?;

        Ok(Self { virtual_output })
    }
}

impl MidiOutput for VirtualMidiOutput {
    fn tick(&mut self) {
        if let Err(err) = self.virtual_output.send(&MIDI_CLOCK_MESSAGE) {
            error!("unable to send TimingClock to virtual output: {err:?}");
        }
    }

    fn play(&mut self) {
        info!("Sending Play");
        self.virtual_output.send(&MIDI_PLAY_MESSAGE).log_error_msg("unable to send play to virtual output").ok();
        self.sysex("PLAY");
    }

    fn stop(&mut self) {
        info!("Sending Stop");
        self.virtual_output.send(&MIDI_STOP_MESSAGE).log_error_msg("unable to send stop to virtual output").ok();
        self.sysex("STOP");
    }

    fn cc(&mut self, channel: Channel, cc: ControlFunction, value: U7) {
        info!("Sending channel {} cc {} value {}", channel.index(), u8::from(cc.0), u8::from(value));
        let mut message = [0; 3];
        MidiMessage::ControlChange(channel, cc, value).copy_to_slice(&mut message).unwrap();
        if let Err(err) = self.virtual_output.send(&message) {
            error!("unable to send cc to virtual output: {err:?}");
        }
    }

    fn sysex(&mut self, value: &str) {
        info!("Sending as sysex: {value}");
        if let Err(err) = self
            .virtual_output
            .send(&[0xF0].into_iter().chain(value.as_bytes().iter().copied()).chain([0xF7]).collect::<Vec<_>>())
        {
            error!("unable to send sysex to virtual output: {err:?}");
        }
    }
}
