use chrono::Duration;

use super::*;

#[test]
fn timing_clock_is_not_forwarded_to_bpm_worker() {
    let event = TimedEvent { timestamp: Duration::zero(), event: MidiMessage::TimingClock };

    assert!(BpmWorkerCommand::try_from(event).is_err());
}
