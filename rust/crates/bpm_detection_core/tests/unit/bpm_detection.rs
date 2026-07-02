use parameter::OnOff;

use super::*;
use crate::{
    note_events::NoteOn,
    parameters::{
        DynamicBPMDetectionConfig, NormalDistributionConfig, StaticBPMDetectionConfig, beat_duration_to_bpm,
        bpm_to_beat_duration,
    },
};

const BPM_TOLERANCE: f32 = 1.0;

fn timed_note_on_at(timestamp: Duration) -> TimedNoteOn {
    TimedNoteOn { timestamp, event: NoteOn { channel: 0, pitch: 60, velocity: 100 } }
}

fn scoring_static_config() -> StaticBPMDetectionConfig {
    StaticBPMDetectionConfig {
        bpm_center: 90.0,
        bpm_range: 40,
        sample_rate: 1_000,
        normal_distribution: NormalDistributionConfig::default(),
    }
}

fn normal_distribution_only_dynamic_config() -> DynamicBPMDetectionConfig {
    DynamicBPMDetectionConfig {
        beats_lookback: 8,
        normal_distribution_weight: OnOff::On(1.0),
        time_distance_weight: OnOff::Off(1.0),
        velocity_current_note_weight: OnOff::Off(1.0),
        velocity_note_from_weight: OnOff::Off(1.0),
        in_beat_range_weight: OnOff::Off(1.0),
        multiplier_weight: OnOff::Off(1.0),
        subdivision_weight: OnOff::Off(1.0),
        octave_distance_weight: OnOff::Off(1.0),
        pitch_distance_weight: OnOff::Off(1.0),
        high_tempo_bias_weight: OnOff::Off(1.0),
    }
}

fn compute_bpm_for_interval(interval: Duration) -> (Vec<f32>, f32) {
    let mut bpm_detection = BPMDetection::new(scoring_static_config());
    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));
    bpm_detection.receive_note_on(timed_note_on_at(interval));

    let (histogram, bpm) = bpm_detection
        .compute_bpm(&normal_distribution_only_dynamic_config())
        .expect("two note-on events with elapsed time should produce a BPM");

    (histogram.to_vec(), bpm)
}

fn assert_successful_scoring(interval: Duration, expected_normalized_interval: Duration) {
    let (histogram, bpm) = compute_bpm_for_interval(interval);
    let positive_bins = histogram.iter().filter(|weight| **weight > 0.0).count();
    let expected_bpm = beat_duration_to_bpm(expected_normalized_interval);

    assert!(positive_bins > 0, "expected scoring to write positive histogram bins");
    assert!(
        (bpm - expected_bpm).abs() <= BPM_TOLERANCE,
        "expected BPM {bpm} to stay within {BPM_TOLERANCE} of normalized interval BPM {expected_bpm}",
    );
}

#[test]
fn compute_bpm_requires_at_least_two_note_on_events() {
    let mut bpm_detection = BPMDetection::new(StaticBPMDetectionConfig::default());

    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));

    assert!(bpm_detection.compute_bpm(&DynamicBPMDetectionConfig::default()).is_none());
}

#[test]
fn compute_bpm_scores_in_range_note_interval() {
    let beat_duration = bpm_to_beat_duration(90.0);

    assert_successful_scoring(beat_duration, beat_duration);
}

#[test]
fn compute_bpm_normalizes_short_interval_by_subdivision() {
    let normalized_beat_duration = bpm_to_beat_duration(90.0);
    let short_interval = bpm_to_beat_duration(180.0);

    assert_successful_scoring(short_interval, normalized_beat_duration);
}

#[test]
fn compute_bpm_normalizes_long_interval_by_multiplier() {
    let normalized_beat_duration = bpm_to_beat_duration(90.0);
    let long_interval = bpm_to_beat_duration(45.0);

    assert_successful_scoring(long_interval, normalized_beat_duration);
}

#[test]
fn compute_bpm_requires_elapsed_time_between_note_on_events() {
    let mut bpm_detection = BPMDetection::new(StaticBPMDetectionConfig::default());

    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));
    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));

    assert!(bpm_detection.compute_bpm(&DynamicBPMDetectionConfig::default()).is_none());
}
