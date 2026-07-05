use bpm_detection_config::{
    DynamicBPMDetectionConfig, NormalDistributionConfig, StaticBPMDetectionConfig, beat_duration_to_bpm,
    bpm_to_beat_duration,
};
use parameter_on_off::OnOff;

use super::*;
use crate::note_events::NoteOn;

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

fn metadata_weight_dynamic_config(
    multiplier_weight: OnOff<f32>,
    subdivision_weight: OnOff<f32>,
) -> DynamicBPMDetectionConfig {
    DynamicBPMDetectionConfig {
        beats_lookback: 8,
        normal_distribution_weight: OnOff::Off(1.0),
        time_distance_weight: OnOff::Off(1.0),
        velocity_current_note_weight: OnOff::Off(1.0),
        velocity_note_from_weight: OnOff::Off(1.0),
        in_beat_range_weight: OnOff::Off(1.0),
        multiplier_weight,
        subdivision_weight,
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

fn score_sum_for_interval(interval: Duration, dynamic_config: &DynamicBPMDetectionConfig) -> f32 {
    let mut bpm_detection = BPMDetection::new(scoring_static_config());
    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));
    bpm_detection.receive_note_on(timed_note_on_at(interval));

    let (histogram, _) = bpm_detection
        .compute_bpm(dynamic_config)
        .expect("two note-on events with elapsed time should produce histogram scores");

    histogram.iter().sum()
}

fn assert_score_unchanged(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON, "expected score sum {actual} to match baseline {expected}");
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON, "expected {actual} to match {expected}");
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

fn fold_candidate(
    observed_note_interval: Duration,
    shortest_candidate_beat_duration: Duration,
    longest_candidate_beat_duration: Duration,
) -> IntervalCandidate {
    fold_observed_interval_into_candidate_beat_range(
        observed_note_interval,
        shortest_candidate_beat_duration,
        longest_candidate_beat_duration,
    )
    .expect("interval should fold into a candidate")
}

#[test]
fn range_folding_keeps_observed_interval_in_range() {
    let candidate =
        fold_candidate(Duration::milliseconds(750), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(750));
    assert_f32_eq(candidate.in_range_score_input, 1.0);
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_keeps_shortest_candidate_beat_duration_in_range() {
    let candidate =
        fold_candidate(Duration::milliseconds(500), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(500));
    assert_f32_eq(candidate.in_range_score_input, 1.0);
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_keeps_longest_candidate_beat_duration_in_range() {
    let candidate =
        fold_candidate(Duration::milliseconds(1_000), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(1_000));
    assert_f32_eq(candidate.in_range_score_input, 1.0);
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_divides_exact_long_boundary_to_longest_candidate_beat_duration() {
    let candidate =
        fold_candidate(Duration::milliseconds(2_000), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(1_000));
    assert!(candidate.in_range_score_input.is_nan());
    assert_f32_eq(candidate.multiple_beat_score_input, 1.0);
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_divides_exact_multi_fold_long_boundary_to_longest_candidate_beat_duration() {
    let candidate =
        fold_candidate(Duration::milliseconds(4_000), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(1_000));
    assert!(candidate.in_range_score_input.is_nan());
    assert_f32_eq(candidate.multiple_beat_score_input, 0.5);
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_divides_long_interval_just_under_power_of_two_ratio() {
    let candidate =
        fold_candidate(Duration::milliseconds(3_999), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::microseconds(999_750));
    assert!(candidate.in_range_score_input.is_nan());
    assert_f32_eq(candidate.multiple_beat_score_input, 0.5);
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_divides_long_interval_just_over_power_of_two_ratio() {
    let candidate =
        fold_candidate(Duration::milliseconds(4_001), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::microseconds(500_125));
    assert!(candidate.in_range_score_input.is_nan());
    assert_f32_eq(candidate.multiple_beat_score_input, 0.25);
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_multiplies_exact_short_boundary_to_shortest_candidate_beat_duration() {
    let candidate =
        fold_candidate(Duration::milliseconds(250), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(500));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert_f32_eq(candidate.subdivision_score_input, 1.0);
}

#[test]
fn range_folding_multiplies_exact_multi_fold_short_boundary_to_shortest_candidate_beat_duration() {
    let candidate =
        fold_candidate(Duration::milliseconds(125), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(500));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert_f32_eq(candidate.subdivision_score_input, 0.5);
}

#[test]
fn range_folding_multiplies_short_interval_just_under_power_of_two_ratio() {
    let candidate =
        fold_candidate(Duration::milliseconds(124), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(992));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert_f32_eq(candidate.subdivision_score_input, 0.25);
}

#[test]
fn range_folding_multiplies_short_interval_just_over_power_of_two_ratio() {
    let candidate =
        fold_candidate(Duration::milliseconds(126), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(504));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert_f32_eq(candidate.subdivision_score_input, 0.5);
}

#[test]
fn range_folding_ignores_one_millisecond_or_smaller_note_clusters() {
    assert!(
        fold_observed_interval_into_candidate_beat_range(
            Duration::microseconds(999),
            Duration::milliseconds(500),
            Duration::milliseconds(1_000),
        )
        .is_none()
    );
    assert!(
        fold_observed_interval_into_candidate_beat_range(
            Duration::milliseconds(1),
            Duration::milliseconds(500),
            Duration::milliseconds(1_000),
        )
        .is_none()
    );
}

#[test]
fn range_folding_ignores_duration_arithmetic_overflow() {
    assert!(
        fold_observed_interval_into_candidate_beat_range(Duration::MAX / 511, Duration::MAX, Duration::MAX).is_none()
    );
}

#[test]
fn range_folding_preserves_short_interval_fold_cap() {
    let candidate =
        fold_candidate(Duration::microseconds(1_001), Duration::milliseconds(600), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::microseconds(512_512));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_divides_long_observed_interval_as_multiple_beat_score_input() {
    let candidate =
        fold_candidate(Duration::milliseconds(1_600), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(800));
    assert!(candidate.in_range_score_input.is_nan());
    assert_f32_eq(candidate.multiple_beat_score_input, 1.0);
    assert!(candidate.subdivision_score_input.is_nan());
}

#[test]
fn range_folding_multiplies_short_observed_interval_as_subdivision_score_input() {
    let candidate =
        fold_candidate(Duration::milliseconds(200), Duration::milliseconds(500), Duration::milliseconds(1_000));

    assert_eq!(candidate.beat_duration, Duration::milliseconds(800));
    assert!(candidate.in_range_score_input.is_nan());
    assert!(candidate.multiple_beat_score_input.is_nan());
    assert_f32_eq(candidate.subdivision_score_input, 0.5);
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
fn compute_bpm_scores_long_interval_multiplier_metadata_with_multiplier_weight() {
    let long_interval = bpm_to_beat_duration(45.0);
    let baseline =
        score_sum_for_interval(long_interval, &metadata_weight_dynamic_config(OnOff::Off(1.0), OnOff::Off(1.0)));
    let with_multiplier =
        score_sum_for_interval(long_interval, &metadata_weight_dynamic_config(OnOff::On(1.0), OnOff::Off(1.0)));
    let with_subdivision =
        score_sum_for_interval(long_interval, &metadata_weight_dynamic_config(OnOff::Off(1.0), OnOff::On(1.0)));

    assert!(with_multiplier > baseline * 9.0);
    assert_score_unchanged(with_subdivision, baseline);
}

#[test]
fn compute_bpm_scores_short_interval_subdivision_metadata_with_subdivision_weight() {
    let short_interval = bpm_to_beat_duration(180.0);
    let baseline =
        score_sum_for_interval(short_interval, &metadata_weight_dynamic_config(OnOff::Off(1.0), OnOff::Off(1.0)));
    let with_multiplier =
        score_sum_for_interval(short_interval, &metadata_weight_dynamic_config(OnOff::On(1.0), OnOff::Off(1.0)));
    let with_subdivision =
        score_sum_for_interval(short_interval, &metadata_weight_dynamic_config(OnOff::Off(1.0), OnOff::On(1.0)));

    assert_score_unchanged(with_multiplier, baseline);
    assert!(with_subdivision > baseline * 9.0);
}

#[test]
fn compute_bpm_requires_elapsed_time_between_note_on_events() {
    let mut bpm_detection = BPMDetection::new(StaticBPMDetectionConfig::default());

    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));
    bpm_detection.receive_note_on(timed_note_on_at(Duration::zero()));

    assert!(bpm_detection.compute_bpm(&DynamicBPMDetectionConfig::default()).is_none());
}
