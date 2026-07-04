use super::*;

#[test]
fn midi_timestamp_to_elapsed_duration_allows_zero_start() {
    assert_eq!(midi_timestamp_to_elapsed_duration(0, 0), Some(Duration::zero()));
}

#[test]
fn midi_timestamp_to_elapsed_duration_returns_elapsed_microseconds() {
    assert_eq!(midi_timestamp_to_elapsed_duration(150, 100), Some(Duration::microseconds(50)));
}

#[test]
fn midi_timestamp_to_elapsed_duration_rejects_timestamp_before_start() {
    assert_eq!(midi_timestamp_to_elapsed_duration(99, 100), None);
}

#[test]
fn midi_timestamp_to_elapsed_duration_rejects_unrepresentable_duration() {
    assert_eq!(midi_timestamp_to_elapsed_duration(u64::MAX - 1, 0), None);
}
