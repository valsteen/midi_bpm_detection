#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

pub use crate::midi_messages::{TimedMidiNoteOn, TimedTypedMidiMessage};

pub mod bpm_detection_receiver;
pub mod midi_messages;
mod normal_distribution;
pub mod parameters;

mod bpm_detection;
mod num_traits_chrono;

pub use bpm_detection::{BPMDetection, NOTE_CAPACITY};
pub use num_traits_chrono::DurationOps;
