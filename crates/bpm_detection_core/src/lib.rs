#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

pub use crate::note_events::{TimedEvent, TimedMidiNoteOn};

pub mod bpm_detection_receiver;
mod normal_distribution;
pub mod note_events;
pub mod parameters;

mod bpm_detection;
mod num_traits_chrono;

pub use bpm_detection::{BPMDetection, NOTE_CAPACITY};
pub use num_traits_chrono::DurationOps;
