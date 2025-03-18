use arraydeque::{ArrayDeque, Wrapping};
use chrono::Duration;
use itertools::Itertools;

use crate::{
    DynamicBPMDetectionParameters, StaticBPMDetectionParameters, TimedMidiNoteOn,
    bpm::{beat_duration_to_bpm, bpm_to_beat_duration, max_histogram_data_buffer_size, sample_to_duration},
    normal_distribution::NormalDistribution,
};

pub const NOTE_CAPACITY: usize = 10000;

pub struct BPMDetection {
    interval_high: Duration,
    interval_low: Duration,
    normal_distribution: NormalDistribution,
    notes: ArrayDeque<TimedMidiNoteOn, NOTE_CAPACITY, Wrapping>,
    static_bpm_detection_parameters: StaticBPMDetectionParameters,
    histogram_data_points: Vec<f32>,
}

impl BPMDetection {
    #[must_use]
    pub fn new(static_bpm_detection_parameters: StaticBPMDetectionParameters) -> Self {
        let mut histogram_data_points = Vec::with_capacity(max_histogram_data_buffer_size());
        histogram_data_points.resize(static_bpm_detection_parameters.buffer_size(), 0.0);
        Self {
            interval_low: bpm_to_beat_duration(static_bpm_detection_parameters.highest_bpm()),
            interval_high: bpm_to_beat_duration(static_bpm_detection_parameters.lowest_bpm()),
            normal_distribution: NormalDistribution::new(static_bpm_detection_parameters.normal_distribution.clone()),
            histogram_data_points,
            static_bpm_detection_parameters,
            notes: ArrayDeque::new(),
        }
    }

    pub fn update_static_parameters(&mut self, static_bpm_detection_parameters: StaticBPMDetectionParameters) {
        self.static_bpm_detection_parameters = static_bpm_detection_parameters;
        self.interval_low = bpm_to_beat_duration(self.static_bpm_detection_parameters.highest_bpm());
        self.interval_high = bpm_to_beat_duration(self.static_bpm_detection_parameters.lowest_bpm());
        self.normal_distribution =
            NormalDistribution::new(self.static_bpm_detection_parameters.normal_distribution.clone());
        self.histogram_data_points.resize(0, 0.0);
        self.histogram_data_points.resize(self.static_bpm_detection_parameters.buffer_size(), 0.0);
    }

    pub fn receive_midi_message(&mut self, midi_message: TimedMidiNoteOn) {
        self.notes.push_back(midi_message);
    }

    pub fn compute_bpm(
        &mut self,
        dynamic_bpm_detection_parameters: &DynamicBPMDetectionParameters,
    ) -> Option<(&[f32], f32)> {
        self.histogram_data_points.fill(0.0);

        let now = self.notes.back()?.timestamp;
        let oldest = self.notes.front()?.timestamp;

        let maximum_interval = now - oldest;

        // consider all combinations of 2 notes, in increasing time order
        self.process_combinations(&now, &maximum_interval, dynamic_bpm_detection_parameters);

        let most_probable_interval = self
            .histogram_data_points
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(index, _)| self.static_bpm_detection_parameters.index_to_duration(index))?;
        let bpm = beat_duration_to_bpm(most_probable_interval);

        let max_note_age = bpm_to_beat_duration(bpm) * i32::from(dynamic_bpm_detection_parameters.beats_lookback);

        loop {
            let Some(note) = self.notes.front() else {
                break;
            };

            if now - note.timestamp > max_note_age {
                self.notes.pop_front();
                continue;
            }
            break;
        }

        Some((&self.histogram_data_points, bpm))
    }

    #[allow(forbidden_lint_groups)]
    #[allow(clippy::too_many_lines)]
    fn process_combinations(
        &mut self,
        newest: &Duration,
        maximum_interval: &Duration,
        dynamic_bpm_detection_parameters: &DynamicBPMDetectionParameters,
    ) {
        for (note_from, note_to) in self.notes.iter().tuple_combinations() {
            let note_age = *newest - note_to.timestamp;
            let mut interval = note_to.timestamp - note_from.timestamp;

            let (interval, in_range, subdivision, multiplier) = {
                let mut in_range: f32 = 1.0;
                let mut subdivision = f32::NAN;
                let mut multiplier = f32::NAN;

                if interval > self.interval_high {
                    in_range = f32::NAN;
                    loop {
                        interval = interval / 2;
                        multiplier = if multiplier.is_nan() { 1.0 } else { multiplier / 2. };
                        if interval < self.interval_high {
                            break;
                        }
                    }
                } else if interval > Duration::milliseconds(1) && interval < self.interval_low {
                    in_range = f32::NAN;
                    for _ in 0..=8 {
                        interval = interval * 2;
                        subdivision = if subdivision.is_nan() { 1.0 } else { subdivision / 2. };
                        if interval > self.interval_low {
                            break;
                        }
                    }
                    if interval < self.interval_low {
                        subdivision = f32::NAN;
                    }
                }

                (interval, in_range, multiplier, subdivision)
            };

            if self
                .static_bpm_detection_parameters
                .duration_to_index(interval, self.histogram_data_points.len())
                .is_none()
            {
                // interval is outside the range of BPM we consider, including trying to multiply or divide the interval
                continue;
            }

            let pitch_distance = 1.
                - f32::from({
                    let interval = (note_to.midi_message.note % 12).abs_diff(note_from.midi_message.note % 12);
                    interval.min(12 - interval)
                }) / 12.0;
            let octave_distance =
                1. - f32::from((note_to.midi_message.note / 12).abs_diff(note_from.midi_message.note / 12)) / 11.; // 11 is approximately the amount of octave that can be represented by midi

            let age = (*maximum_interval - note_age).num_microseconds().unwrap() as f32
                / maximum_interval.num_microseconds().unwrap() as f32;
            let velocity_note_from = f32::from(note_from.midi_message.velocity) / 127.;
            let velocity_current_note = f32::from(note_to.midi_message.velocity) / 127.;

            let high_tempo_bias = {
                let interval_low_num = self.interval_low.num_microseconds().unwrap() as f32;
                let interval_high_num = self.interval_high.num_microseconds().unwrap() as f32;
                let note_interval_num = interval.num_microseconds().unwrap() as f32;
                1.0 - (note_interval_num - interval_low_num) / (interval_high_num - interval_low_num)
            };

            let intensity: f32 = [
                (velocity_current_note, dynamic_bpm_detection_parameters.velocity_current_note_weight.weight()),
                (velocity_note_from, dynamic_bpm_detection_parameters.velocity_note_from_weight.weight()),
                (age, dynamic_bpm_detection_parameters.age_weight.weight()),
                (octave_distance, dynamic_bpm_detection_parameters.octave_distance_weight.weight()),
                (pitch_distance, dynamic_bpm_detection_parameters.pitch_distance_weight.weight()),
                (multiplier, dynamic_bpm_detection_parameters.multiplier_weight.weight()),
                (subdivision, dynamic_bpm_detection_parameters.subdivision_weight.weight()),
                (in_range, dynamic_bpm_detection_parameters.in_beat_range_weight.weight()),
                (high_tempo_bias, dynamic_bpm_detection_parameters.high_tempo_bias.weight()),
            ]
            .into_iter()
            // We normalize the value to be between 1 and 10, so log10 will give a value between 0 and 1,
            .map(|(c, w)| (c * 9.0 + 1.0).log10() * w)
            .filter(|criteria| criteria.is_finite() && *criteria > 0.0)
            .sum();

            let imprecision = Duration::nanoseconds(
                (self.normal_distribution.normal_distribution_config.imprecision * 1_000_000.0) as i64,
            );
            let duration_per_sample = sample_to_duration(self.static_bpm_detection_parameters.sample_rate, 1);
            let mut timestamp = -imprecision;
            let normal_weight = dynamic_bpm_detection_parameters.normal_distribution_weight.weight();
            while timestamp <= imprecision {
                if let Some(index) = self
                    .static_bpm_detection_parameters
                    .duration_to_index(timestamp + interval, self.histogram_data_points.len())
                {
                    let normal_value = if normal_weight > 0.0 {
                        (self.normal_distribution[timestamp]
                            * 9.0
                            // the normal distribution will have values up to 4, this adjusts to be around the same range
                            * 2.0
                            + 1.0)
                            .log10()
                            * dynamic_bpm_detection_parameters.normal_distribution_weight.weight()
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
