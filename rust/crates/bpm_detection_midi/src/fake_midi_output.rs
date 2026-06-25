// This module only exists to allow building to a wasm target, which does not support Virtual midi output
#![cfg(not(unix))]
use errors::Result;
use log::info;
use wmidi::{Channel, ControlFunction, MidiMessage, U7};

use crate::midi_output_trait::MidiOutput;

pub struct VirtualMidiOutput {}

impl VirtualMidiOutput {
    #[allow(clippy::unnecessary_wraps)]
    pub fn new(_device_name: &str) -> Result<Self> {
        Ok(Self {})
    }
}

impl MidiOutput for VirtualMidiOutput {
    fn tick(&mut self) {}

    fn play(&mut self) {
        info!("Sending Play");

        self.sysex("PLAY");
    }

    fn stop(&mut self) {
        info!("Sending Stop");

        self.sysex("STOP");
    }

    fn cc(&mut self, channel: Channel, cc: ControlFunction, value: U7) {
        info!("Sending channel {} cc {} value {}", channel.index(), u8::from(cc.0), u8::from(value));
        let mut message = [0; 3];
        MidiMessage::ControlChange(channel, cc, value).copy_to_slice(&mut message).unwrap();
    }

    fn sysex(&mut self, value: &str) {
        info!("Sending as sysex: {value}");
    }
}
