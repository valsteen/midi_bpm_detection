#![allow(dead_code)]

use wmidi::{Channel, ControlFunction, U7};

pub const MIDI_CLOCK_MESSAGE: [u8; 1] = [0xF8];
pub const MIDI_CONTINUE_MESSAGE: [u8; 1] = [0xFB];
pub const MIDI_PLAY_MESSAGE: [u8; 1] = [0xFA];
pub const MIDI_STOP_MESSAGE: [u8; 1] = [0xFC];

pub trait MidiOutput {
    fn tick(&mut self);
    fn play(&mut self);
    fn stop(&mut self);
    fn cc(&mut self, channel: Channel, cc: ControlFunction, value: U7);
    fn sysex(&mut self, value: &str);
}
