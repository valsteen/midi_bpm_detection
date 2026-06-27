use bpm_detection_core::{
    TimedEvent, TimedNoteOn,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use wmidi::MidiMessage;

use crate::midi_note_on_from_message;

/// Narrow command protocol for the native BPM worker mailbox.
///
/// This is intentionally not a desktop-wide event bus. It contains only the messages the worker owns: note-on
/// observations, detection config updates, and transport commands forwarded to the MIDI output thread.
pub enum BpmWorkerCommand {
    TimedNoteOn(TimedNoteOn),
    Play,
    Stop,
    DynamicBPMDetectionConfig(DynamicBPMDetectionConfig),
    StaticBPMDetectionConfig(StaticBPMDetectionConfig),
}

impl TryFrom<TimedEvent<MidiMessage<'_>>> for BpmWorkerCommand {
    type Error = ();

    fn try_from(value: TimedEvent<MidiMessage<'_>>) -> Result<Self, Self::Error> {
        Ok(Self::TimedNoteOn(TimedNoteOn {
            timestamp: value.timestamp,
            event: midi_note_on_from_message(&value.event).ok_or(())?,
        }))
    }
}

#[cfg(test)]
#[path = "../tests/unit/worker_command.rs"]
mod tests;
