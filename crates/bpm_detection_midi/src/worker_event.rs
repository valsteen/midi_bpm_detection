use bpm_detection_core::{
    TimedEvent, TimedNoteOn,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use wmidi::MidiMessage;

use crate::midi_note_on_from_message;

pub enum WorkerEvent {
    TimedNoteOn(TimedNoteOn),
    Play,
    Stop,
    DynamicBPMDetectionConfig(DynamicBPMDetectionConfig),
    StaticBPMDetectionConfig(StaticBPMDetectionConfig),
}

impl TryFrom<TimedEvent<MidiMessage<'_>>> for WorkerEvent {
    type Error = ();

    fn try_from(value: TimedEvent<MidiMessage<'_>>) -> Result<Self, Self::Error> {
        Ok(Self::TimedNoteOn(TimedNoteOn {
            timestamp: value.timestamp,
            event: midi_note_on_from_message(value.event).ok_or(())?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn timing_clock_is_not_forwarded_to_bpm_worker() {
        let event = TimedEvent { timestamp: Duration::zero(), event: MidiMessage::TimingClock };

        assert!(WorkerEvent::try_from(event).is_err());
    }
}
