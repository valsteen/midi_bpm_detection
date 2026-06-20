use bpm_detection_core::{
    TimedMidiNoteOn, TimedTypedMidiMessage,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use wmidi::MidiMessage;

use crate::midi_note_on_from_message;

pub enum WorkerEvent {
    TimedMidiNoteOn(TimedMidiNoteOn),
    TimingClock,
    Play,
    Stop,
    DynamicBPMDetectionConfig(DynamicBPMDetectionConfig),
    StaticBPMDetectionConfig(StaticBPMDetectionConfig),
}

impl TryFrom<TimedTypedMidiMessage<MidiMessage<'_>>> for WorkerEvent {
    type Error = ();

    fn try_from(value: TimedTypedMidiMessage<MidiMessage<'_>>) -> Result<Self, Self::Error> {
        if let MidiMessage::TimingClock = value.midi_message {
            return Ok(Self::TimingClock);
        }
        Ok(Self::TimedMidiNoteOn(TimedMidiNoteOn {
            timestamp: value.timestamp,
            midi_message: midi_note_on_from_message(value.midi_message).ok_or(())?,
        }))
    }
}
