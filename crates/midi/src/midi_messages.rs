use crate::StaticMidiMessage;
use chrono::Duration;

use std::fmt::{Debug, Display};
pub use wmidi;

#[derive(Eq, PartialEq, Clone)]
pub struct TimedTypedMidiMessage<T> {
    pub timestamp: Duration,
    pub midi_message: T,
}

pub type TimedMidiMessage = TimedTypedMidiMessage<StaticMidiMessage>;
pub type TimedMidiNoteOn = TimedTypedMidiMessage<MidiNoteOn>;

pub struct MidiNoteOn {
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
}

impl TryFrom<TimedTypedMidiMessage<StaticMidiMessage>> for TimedMidiNoteOn {
    type Error = ();

    fn try_from(value: TimedTypedMidiMessage<StaticMidiMessage>) -> Result<Self, Self::Error> {
        Ok(Self { timestamp: value.timestamp, midi_message: value.midi_message.try_into()? })
    }
}

impl TryFrom<StaticMidiMessage> for MidiNoteOn {
    type Error = ();

    fn try_from(value: StaticMidiMessage) -> Result<Self, Self::Error> {
        if let StaticMidiMessage::NoteOn(channel, note, velocity) = value {
            return Ok(Self { channel: channel.index(), note: note as u8, velocity: u8::from(velocity) });
        }
        Err(())
    }
}

impl<T> Debug for TimedTypedMidiMessage<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_seconds = self.timestamp.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let milliseconds = self.timestamp.subsec_nanos() / 1_000_000;
        Display::fmt(&format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03} {:?}", self.midi_message), f)
    }
}
