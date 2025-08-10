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
pub use midi_in::{MidiIn, MidiService};
pub use midi_messages::TimedMidiMessage;
pub use midir::{MidiInput, MidiInputConnection};
pub use wmidi::MidiMessage;
pub type StaticMidiMessage = MidiMessage<'static>;
pub type MidiError = wmidi::Error;

pub use crate::midi_messages::{TimedMidiNoteOn, TimedTypedMidiMessage};

pub mod bpm_detection_receiver;
pub mod midi_in;
pub mod midi_messages;
mod midi_output;
mod normal_distribution;
pub mod parameters;
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
pub use sysex::SysExCommand;

pub use crate::midi_input_port::MidiInputPort;
