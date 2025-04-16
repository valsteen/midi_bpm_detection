#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::marker::PhantomData;

pub use bpm::{
    DefaultDynamicBPMDetectionParameters, DefaultStaticBPMDetectionParameters, DynamicBPMDetectionConfig,
    StaticBPMDetectionConfig,
};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use coremidi::restart;
use derivative::Derivative;
pub use midi_in::{MidiIn, MidiService};
pub use midi_messages::TimedMidiMessage;
pub use midir::{MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
pub use wmidi::MidiMessage;
pub type StaticMidiMessage = MidiMessage<'static>;
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
use parameter::Parameter;
use sync::ArcAtomicBool;
pub use sysex::SysExCommand;

pub use crate::midi_input_port::MidiInputPort;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiServiceConfig {
    pub device_name: String,
    pub send_tempo: ArcAtomicBool,
    pub enable_midi_clock: ArcAtomicBool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Eq)]
#[serde(default)]
pub struct NormalDistributionConfig {
    #[derivative(PartialEq(compare_with = "f64::eq"))]
    pub std_dev: f64,
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub factor: f32,
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub cutoff: f32, // in millisecond
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub resolution: f32, // 1 means one index = 1 millisecond
}

pub trait NormalDistributionConfigAccessor {
    fn std_dev(&self) -> f64;
    fn factor(&self) -> f32;
    fn cutoff(&self) -> f32;
    fn resolution(&self) -> f32;

    fn set_std_dev(&mut self, val: f64);
    fn set_factor(&mut self, val: f32);
    fn set_cutoff(&mut self, val: f32);
    fn set_resolution(&mut self, val: f32);
}

impl NormalDistributionConfigAccessor for () {
    fn std_dev(&self) -> f64 {
        unimplemented!()
    }

    fn factor(&self) -> f32 {
        unimplemented!()
    }

    fn cutoff(&self) -> f32 {
        unimplemented!()
    }

    fn resolution(&self) -> f32 {
        unimplemented!()
    }

    fn set_std_dev(&mut self, _: f64) {
        unimplemented!()
    }

    fn set_factor(&mut self, _: f32) {
        unimplemented!()
    }

    fn set_cutoff(&mut self, _: f32) {
        unimplemented!()
    }

    fn set_resolution(&mut self, _: f32) {
        unimplemented!()
    }
}

pub type DefaultNormalDistributionParameters = NormalDistributionParameters<()>;

impl Default for NormalDistributionConfig {
    fn default() -> Self {
        Self {
            std_dev: DefaultNormalDistributionParameters::STD_DEV.default,
            factor: DefaultNormalDistributionParameters::FACTOR.default,
            cutoff: DefaultNormalDistributionParameters::CUTOFF.default,
            resolution: DefaultNormalDistributionParameters::RESOLUTION.default,
        }
    }
}

pub struct NormalDistributionParameters<Config: NormalDistributionConfigAccessor> {
    phantom: PhantomData<Config>,
}

impl<Config: NormalDistributionConfigAccessor> NormalDistributionParameters<Config> {
    pub const CUTOFF: Parameter<Config, f32> = Parameter::new(
        "Normal distribution cutoff",
        Some("ms"),
        1.0..=2000.,
        0.0,
        true,
        100.0,
        Config::cutoff,
        Config::set_cutoff,
    );
    pub const FACTOR: Parameter<Config, f32> =
        Parameter::new("factor", None, 0.0..=50., 0.0, false, 40.0, Config::factor, Config::set_factor);
    pub const RESOLUTION: Parameter<Config, f32> = Parameter::new(
        "Normal distribution resolution",
        Some("ms"),
        0.01..=1000.,
        0.0,
        true,
        0.6,
        Config::resolution,
        Config::set_resolution,
    );
    pub const STD_DEV: Parameter<Config, f64> =
        Parameter::new("Standard deviation", None, 4.0..=40.0, 0.0, false, 24.0, Config::std_dev, Config::set_std_dev);
}
