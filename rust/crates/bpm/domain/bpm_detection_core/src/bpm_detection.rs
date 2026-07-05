use std::time::Duration as StdDuration;

use arraydeque::{ArrayDeque, Wrapping};
use bpm_detection_config::{
    DynamicBPMDetectionConfig, StaticBPMDetectionConfig, beat_duration_to_bpm, bpm_to_beat_duration,
    max_histogram_data_buffer_size, sample_to_duration,
};
use chrono::Duration;
use itertools::Itertools;

use crate::{TimedNoteOn, normal_distribution::NormalDistribution};

pub const NOTE_CAPACITY: usize = 10000;

const NANOS_PER_SECOND: u128 = 1_000_000_000;
const SHORT_INTERVAL_MAX_FOLD_COUNT: u32 = 9;

struct IntervalCandidate {
    beat_duration: Duration,
    in_range_score_input: f32,
    multiple_beat_score_input: f32,
    subdivision_score_input: f32,
}

// The detector is looking for BPM, but it scores beat-duration candidates.
//
// For each pair of note-on events, the raw material is the observed note interval:
// the elapsed duration between those two events. If that duration is outside the
// configured BPM window, it may still imply an accepted beat duration after folding
// by powers of two. With an inclusive accepted beat-duration range of 500ms..1000ms (120..60 BPM):
//
// - 750ms (80 BPM) stays 750ms (80 BPM): already in range.
// - 1600ms (37.5 BPM) is divided to 800ms (75 BPM): the notes may span multiple beats.
// - 200ms (300 BPM) is multiplied to 800ms (75 BPM): the notes may be beat subdivisions.
//
// The returned score inputs are not configured weights and not final scores. The scoring
// loop applies the corresponding `*_weight` after log-normalizing each finite input.
fn fold_observed_interval_into_candidate_beat_range(
    mut observed_note_interval: Duration,
    shortest_candidate_beat_duration: Duration,
    longest_candidate_beat_duration: Duration,
) -> Option<IntervalCandidate> {
    let mut in_range_score_input: f32 = 1.0;
    let mut subdivision_score_input = f32::NAN;
    let mut multiple_beat_score_input = f32::NAN;

    if observed_note_interval > longest_candidate_beat_duration {
        in_range_score_input = f32::NAN;
        let fold_count = power_of_two_fold_count(observed_note_interval, longest_candidate_beat_duration)?;
        observed_note_interval = divide_duration_by_power_of_two(observed_note_interval, fold_count)?;
        multiple_beat_score_input = folded_score_input(fold_count);
    } else if observed_note_interval < shortest_candidate_beat_duration {
        if observed_note_interval <= Duration::milliseconds(1) {
            return None;
        }
        in_range_score_input = f32::NAN;
        let required_fold_count = power_of_two_fold_count(shortest_candidate_beat_duration, observed_note_interval)?;
        let applied_fold_count = required_fold_count.min(SHORT_INTERVAL_MAX_FOLD_COUNT);
        observed_note_interval = multiply_duration_by_power_of_two(observed_note_interval, applied_fold_count)?;
        if required_fold_count <= SHORT_INTERVAL_MAX_FOLD_COUNT {
            subdivision_score_input = folded_score_input(applied_fold_count);
        }
    }

    Some(IntervalCandidate {
        beat_duration: observed_note_interval,
        in_range_score_input,
        multiple_beat_score_input,
        subdivision_score_input,
    })
}

fn power_of_two_fold_count(numerator: Duration, denominator: Duration) -> Option<u32> {
    let numerator = positive_duration_nanoseconds(numerator)?;
    let denominator = positive_duration_nanoseconds(denominator)?;
    if denominator == 0 {
        return None;
    }

    Some(ceil_log2(numerator.div_ceil(denominator)))
}

fn positive_duration_nanoseconds(duration: Duration) -> Option<u128> {
    duration.to_std().ok().map(|duration| duration.as_nanos())
}

fn ceil_log2(value: u128) -> u32 {
    // Integer ceil(log2(value)): subtract one so exact powers of two do not round up.
    // The remaining bit width is the number of power-of-two folds needed to cover `value`.
    u128::BITS - value.saturating_sub(1).leading_zeros()
}

fn divide_duration_by_power_of_two(duration: Duration, fold_count: u32) -> Option<Duration> {
    let fold_factor = power_of_two_factor(fold_count)?;
    let nanoseconds = positive_duration_nanoseconds(duration)?;

    duration_from_nanoseconds(nanoseconds / fold_factor)
}

fn multiply_duration_by_power_of_two(duration: Duration, fold_count: u32) -> Option<Duration> {
    let fold_factor = power_of_two_factor(fold_count)?;
    let nanoseconds = positive_duration_nanoseconds(duration)?;

    duration_from_nanoseconds(nanoseconds.checked_mul(fold_factor)?)
}

fn power_of_two_factor(fold_count: u32) -> Option<u128> {
    1_u128.checked_shl(fold_count)
}

fn duration_from_nanoseconds(nanoseconds: u128) -> Option<Duration> {
    let seconds = u64::try_from(nanoseconds / NANOS_PER_SECOND).ok()?;
    let subsecond_nanoseconds = u32::try_from(nanoseconds % NANOS_PER_SECOND).ok()?;
    let duration = StdDuration::new(seconds, subsecond_nanoseconds);

    Duration::from_std(duration).ok()
}

fn folded_score_input(fold_count: u32) -> f32 {
    let exponent = fold_count.saturating_sub(1).min(i32::MAX as u32) as i32;

    2.0_f32.powi(-exponent)
}

fn duration_to_histogram_index(
    config: &StaticBPMDetectionConfig,
    duration: Duration,
    buffer_size: usize,
) -> Option<usize> {
    let index = config
        .duration_to_sample(duration)
        .checked_sub(config.duration_to_sample(bpm_to_beat_duration(config.highest_bpm())))?;
    (index < buffer_size).then_some(index)
}

fn histogram_index_to_duration(config: &StaticBPMDetectionConfig, index: usize) -> Duration {
    sample_to_duration(config.sample_rate, index) + bpm_to_beat_duration(config.highest_bpm())
}

pub struct BPMDetection {
    interval_high: Duration,
    interval_low: Duration,
    normal_distribution: NormalDistribution,
    note_events: ArrayDeque<TimedNoteOn, NOTE_CAPACITY, Wrapping>,
    static_bpm_detection_config: StaticBPMDetectionConfig,
    histogram_data_points: Vec<f32>,
}

impl BPMDetection {
    #[must_use]
    pub fn new(static_bpm_detection_config: StaticBPMDetectionConfig) -> Self {
        let mut histogram_data_points = Vec::with_capacity(max_histogram_data_buffer_size());
        histogram_data_points.resize(static_bpm_detection_config.buffer_size(), 0.0);
        Self {
            interval_low: bpm_to_beat_duration(static_bpm_detection_config.highest_bpm()),
            interval_high: bpm_to_beat_duration(static_bpm_detection_config.lowest_bpm()),
            normal_distribution: NormalDistribution::new(static_bpm_detection_config.normal_distribution.clone()),
            histogram_data_points,
            static_bpm_detection_config,
            note_events: ArrayDeque::new(),
        }
    }

    pub fn update_static_config(&mut self, static_bpm_detection_config: StaticBPMDetectionConfig) {
        self.static_bpm_detection_config = static_bpm_detection_config;
        self.interval_low = bpm_to_beat_duration(self.static_bpm_detection_config.highest_bpm());
        self.interval_high = bpm_to_beat_duration(self.static_bpm_detection_config.lowest_bpm());
        self.normal_distribution =
            NormalDistribution::new(self.static_bpm_detection_config.normal_distribution.clone());
        self.histogram_data_points.clear();
        self.histogram_data_points.resize(self.static_bpm_detection_config.buffer_size(), 0.0);
    }

    pub fn receive_note_on(&mut self, event: TimedNoteOn) {
        self.note_events.push_back(event);
    }

    pub fn compute_bpm(&mut self, dynamic_bpm_detection_config: &DynamicBPMDetectionConfig) -> Option<(&[f32], f32)> {
        self.histogram_data_points.fill(0.0);

        if self.note_events.len() < 2 {
            return None;
        }
        let now = self.note_events.back()?.timestamp;
        let oldest = self.note_events.front()?.timestamp;
        let maximum_interval = now - oldest;
        if maximum_interval <= Duration::zero() {
            return None;
        }

        // Consider all note-on pairs, in increasing time order.
        self.process_combinations(&now, &maximum_interval, dynamic_bpm_detection_config);

        let most_probable_interval = self
            .histogram_data_points
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .filter(|(_, weight)| **weight > 0.0)
            .map(|(index, _)| histogram_index_to_duration(&self.static_bpm_detection_config, index))?;
        let bpm = beat_duration_to_bpm(most_probable_interval);

        let max_note_age = bpm_to_beat_duration(bpm) * i32::from(dynamic_bpm_detection_config.beats_lookback);

        while let Some(note) = self.note_events.front() {
            if now - note.timestamp > max_note_age {
                self.note_events.pop_front();
                continue;
            }
            break;
        }

        Some((&self.histogram_data_points, bpm))
    }

    fn process_combinations(
        &mut self,
        newest: &Duration,
        maximum_interval: &Duration,
        dynamic_bpm_detection_config: &DynamicBPMDetectionConfig,
    ) {
        let cutoff =
            Duration::nanoseconds((self.normal_distribution.normal_distribution_config.cutoff * 1_000_000.0) as i64);
        let duration_per_sample = sample_to_duration(self.static_bpm_detection_config.sample_rate, 1);
        let normal_weight = dynamic_bpm_detection_config.normal_distribution_weight.weight();

        for [note_from, note_to] in self.note_events.iter().array_combinations() {
            let note_age = *newest - note_to.timestamp;
            let Some(IntervalCandidate {
                beat_duration: interval,
                in_range_score_input,
                multiple_beat_score_input,
                subdivision_score_input,
            }) = fold_observed_interval_into_candidate_beat_range(
                note_to.timestamp - note_from.timestamp,
                self.interval_low,
                self.interval_high,
            )
            else {
                continue;
            };

            if duration_to_histogram_index(
                &self.static_bpm_detection_config,
                interval,
                self.histogram_data_points.len(),
            )
            .is_none()
            {
                // interval is outside the range of BPM we consider, including trying to multiply or divide the interval
                continue;
            }

            let pitch_distance = 1.
                - f32::from({
                    let interval = (note_to.event.pitch % 12).abs_diff(note_from.event.pitch % 12);
                    interval.min(12 - interval)
                }) / 12.0;
            // 11 is approximately the number of octaves representable by the incoming pitch value.
            let octave_distance = 1. - f32::from((note_to.event.pitch / 12).abs_diff(note_from.event.pitch / 12)) / 11.;

            let age = (*maximum_interval - note_age).num_microseconds().unwrap() as f32
                / maximum_interval.num_microseconds().unwrap() as f32;
            let velocity_note_from = f32::from(note_from.event.velocity) / 127.;
            let velocity_current_note = f32::from(note_to.event.velocity) / 127.;

            let high_tempo_bias_score = {
                let interval_low_num = self.interval_low.num_microseconds().unwrap() as f32;
                let interval_high_num = self.interval_high.num_microseconds().unwrap() as f32;
                let note_interval_num = interval.num_microseconds().unwrap() as f32;
                1.0 - (note_interval_num - interval_low_num) / (interval_high_num - interval_low_num)
            };

            let intensity: f32 = [
                (velocity_current_note, dynamic_bpm_detection_config.velocity_current_note_weight.weight()),
                (velocity_note_from, dynamic_bpm_detection_config.velocity_note_from_weight.weight()),
                (age, dynamic_bpm_detection_config.time_distance_weight.weight()),
                (octave_distance, dynamic_bpm_detection_config.octave_distance_weight.weight()),
                (pitch_distance, dynamic_bpm_detection_config.pitch_distance_weight.weight()),
                (multiple_beat_score_input, dynamic_bpm_detection_config.multiplier_weight.weight()),
                (subdivision_score_input, dynamic_bpm_detection_config.subdivision_weight.weight()),
                (in_range_score_input, dynamic_bpm_detection_config.in_beat_range_weight.weight()),
                (high_tempo_bias_score, dynamic_bpm_detection_config.high_tempo_bias_weight.weight()),
            ]
            .into_iter()
            // We normalize the value to be between 1 and 10, so log10 will give a value between 0 and 1,
            .map(|(c, w)| (c * 9.0 + 1.0).log10() * w)
            .filter(|criteria| criteria.is_finite() && *criteria > 0.0)
            .sum();

            let mut timestamp = -cutoff;
            while timestamp <= cutoff {
                if let Some(index) = duration_to_histogram_index(
                    &self.static_bpm_detection_config,
                    timestamp + interval,
                    self.histogram_data_points.len(),
                ) {
                    let normal_value = if normal_weight > 0.0 {
                        (self.normal_distribution[timestamp]
                            * 9.0
                            // the normal distribution will have values up to 4, this adjusts to be around the same range
                            * 2.0
                            + 1.0)
                            .log10()
                            * normal_weight
                    } else {
                        0.0
                    };

                    self.histogram_data_points[index] += 10.0f32.powf(intensity + normal_value);
                }

                timestamp += duration_per_sample;
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/bpm_detection.rs"]
mod tests;
