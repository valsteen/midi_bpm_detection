use super::*;
use crate::{note_events::NoteOn, parameters::DynamicBPMDetectionConfig};

fn timed_note_on_at(timestamp: Duration) -> TimedNoteOn {
    TimedNoteOn { timestamp, event: NoteOn { channel: 0, pitch: 60, velocity: 100 } }
}

#[test]
fn compute_bpm_requires_at_least_two_note_on_events() {
    let mut bpm_detection = BPMDetection::new(StaticBPMDetectionConfig::default());

    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));

    assert!(bpm_detection.compute_bpm(&DynamicBPMDetectionConfig::default()).is_none());
}

#[test]
fn compute_bpm_requires_elapsed_time_between_note_on_events() {
    let mut bpm_detection = BPMDetection::new(StaticBPMDetectionConfig::default());

    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));
    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));

    assert!(bpm_detection.compute_bpm(&DynamicBPMDetectionConfig::default()).is_none());
}
