#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use coremidi::restart;
use derivative::Derivative;
pub use midi_in::{MidiIn, MidiService};
pub use midi_messages::TimedMidiMessage;
pub use midir::{MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
pub use wmidi::MidiMessage;

pub type StaticMidiMessage = wmidi::MidiMessage<'static>;
pub type MidiError = wmidi::Error;

pub use crate::midi_messages::{TimedMidiNoteOn, TimedTypedMidiMessage};

pub mod bpm;
pub mod bpm_detection_receiver;
pub mod midi_in;
pub mod midi_messages;
mod midi_output;
mod normal_distribution;
mod worker;

mod bpm_detection;
mod fake_midi_output;
mod midi_input_port;
mod midi_output_trait;
mod num_traits_chrono;
mod sysex;
mod worker_event;

pub use bpm_detection::BPMDetection;
pub use num_traits_chrono::DurationOps;
use parameter::{MutGetters, Parameter};
use sync::ArcAtomicBool;
pub use sysex::SysExCommand;

pub use crate::{
    bpm::{DynamicBPMDetectionParameters, StaticBPMDetectionParameters},
    midi_input_port::MidiInputPort,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiServiceConfig {
    pub device_name: String,
    pub send_tempo: ArcAtomicBool,
    pub enable_midi_clock: ArcAtomicBool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Derivative, MutGetters)]
#[derivative(PartialEq, Eq)]
#[getset(get_mut = "pub")]
#[serde(default)]
pub struct NormalDistributionConfig {
    #[derivative(PartialEq(compare_with = "f64::eq"))]
    pub std_dev: f64,
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub factor: f32,
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub imprecision: f32, // in millisecond
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub resolution: f32, // 1 means one index = 1 millisecond
}

impl Default for NormalDistributionConfig {
    fn default() -> Self {
        Self {
            std_dev: Self::STD_DEV.default,
            factor: Self::FACTOR.default,
            imprecision: Self::IMPRECISION.default,
            resolution: Self::RESOLUTION.default,
        }
    }
}

impl NormalDistributionConfig {
    pub const FACTOR: Parameter<Self, f32> =
        Parameter::new("factor", None, 0.0..=50., 0.0, false, 40.0, Self::factor_mut);
    pub const IMPRECISION: Parameter<Self, f32> =
        Parameter::new("Normal distribution cutoff", Some("ms"), 1.0..=2000., 0.0, true, 100.0, Self::imprecision_mut);
    pub const RESOLUTION: Parameter<Self, f32> = Parameter::new(
        "Normal distribution resolution",
        Some("ms"),
        0.01..=1000.,
        0.0,
        true,
        0.6,
        Self::resolution_mut,
    );
    pub const STD_DEV: Parameter<Self, f64> =
        Parameter::new("Standard deviation", None, 4.0..=40.0, 0.0, false, 24.0, Self::std_dev_mut);
}
