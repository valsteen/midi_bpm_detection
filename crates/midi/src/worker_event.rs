use crate::{
    DynamicBPMDetectionParameters, StaticBPMDetectionParameters, StaticMidiMessage, TimedMidiNoteOn,
    TimedTypedMidiMessage,
};
use wmidi::MidiMessage;

pub enum WorkerEvent {
    TimedMidiNoteOn(TimedMidiNoteOn),
    TimingClock,
    Play,
    Stop,
    DynamicBPMDetectionParameters(DynamicBPMDetectionParameters),
    StaticBPMDetectionParameters(StaticBPMDetectionParameters),
}

impl TryFrom<TimedTypedMidiMessage<StaticMidiMessage>> for WorkerEvent {
    type Error = ();

    fn try_from(value: TimedTypedMidiMessage<StaticMidiMessage>) -> errors::Result<Self, Self::Error> {
        if let MidiMessage::TimingClock = value.midi_message {
            return Ok(Self::TimingClock);
        }
        Ok(Self::TimedMidiNoteOn(TimedMidiNoteOn::try_from(value)?))
    }
}
