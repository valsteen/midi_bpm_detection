use wmidi::MidiMessage;

use crate::{
    TimedMidiNoteOn, TimedTypedMidiMessage,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};

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
        Ok(Self::TimedMidiNoteOn(TimedMidiNoteOn::try_from(value)?))
    }
}
