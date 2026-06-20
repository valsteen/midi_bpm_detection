use bpm_detection_core::{
    TimedEvent, TimedMidiNoteOn,
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

impl TryFrom<TimedEvent<MidiMessage<'_>>> for WorkerEvent {
    type Error = ();

    fn try_from(value: TimedEvent<MidiMessage<'_>>) -> Result<Self, Self::Error> {
        if let MidiMessage::TimingClock = value.event {
            return Ok(Self::TimingClock);
        }
        Ok(Self::TimedMidiNoteOn(TimedMidiNoteOn {
            timestamp: value.timestamp,
            event: midi_note_on_from_message(value.event).ok_or(())?,
        }))
    }
}
