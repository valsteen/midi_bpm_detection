#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

use bpm_detection_core::{TimedTypedMidiMessage, midi_messages::MidiNoteOn};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub use coremidi::restart;
pub use midi_in::{MidiIn, MidiService};
pub use midi_input_port::MidiInputPort;
pub use midir::{MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use sync::ArcAtomicBool;
pub use sysex::SysExCommand;
pub use wmidi::{self, MidiMessage};

pub type StaticMidiMessage = MidiMessage<'static>;
pub type TimedMidiMessage = TimedTypedMidiMessage<StaticMidiMessage>;

mod fake_midi_output;
pub mod midi_in;
mod midi_input_port;
mod midi_output;
mod midi_output_trait;
mod sysex;
mod worker;
mod worker_event;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiServiceConfig {
    pub device_name: String,
    pub send_tempo: ArcAtomicBool,
    pub enable_midi_clock: ArcAtomicBool,
}

pub fn midi_note_on_from_message(midi_message: MidiMessage<'_>) -> Option<MidiNoteOn> {
    if let MidiMessage::NoteOn(channel, note, velocity) = midi_message {
        return Some(MidiNoteOn { channel: channel.index(), note: note as u8, velocity: u8::from(velocity) });
    }
    None
}

#[must_use]
pub fn to_owned_midi_message(value: TimedTypedMidiMessage<MidiMessage<'_>>) -> TimedMidiMessage {
    let TimedTypedMidiMessage { timestamp, midi_message } = value;
    TimedTypedMidiMessage { timestamp, midi_message: midi_message.to_owned() }
}
