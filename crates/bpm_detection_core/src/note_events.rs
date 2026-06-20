use std::fmt::{Debug, Display};

use chrono::Duration;

#[derive(Eq, PartialEq, Clone)]
pub struct TimedEvent<T> {
    pub timestamp: Duration,
    pub event: T,
}

pub type TimedNoteOn = TimedEvent<NoteOn>;

pub struct NoteOn {
    pub channel: u8,
    pub pitch: u8,
    pub velocity: u8,
}

impl<T> Debug for TimedEvent<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_seconds = self.timestamp.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let milliseconds = self.timestamp.subsec_nanos() / 1_000_000;
        Display::fmt(&format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03} {:?}", self.event), f)
    }
}
