use crate::StaticMidiMessage;
use std::str::{FromStr, from_utf8};
use wmidi::MidiMessage;

pub enum SysExCommand {
    Tempo(f32),
    Play,
    Stop,
}

impl TryFrom<&StaticMidiMessage> for SysExCommand {
    type Error = ();

    fn try_from(value: &StaticMidiMessage) -> Result<Self, Self::Error> {
        if let MidiMessage::OwnedSysEx(sysex) = value {
            let bytes = sysex.iter().map(|u7| u8::from(*u7)).collect::<Vec<_>>();
            let sysex_string = from_utf8(&bytes).or(Err(()))?;
            let mut parts = sysex_string.splitn(2, '|');
            return Ok(match (parts.next(), parts.next()) {
                (Some("PLAY"), None) => Self::Play, // TODO - search for PLAY STOP , use this instead. also lookup again
                (Some("STOP"), None) => Self::Stop,
                (Some("TEMPO"), Some(rpm)) => {
                    let rpm = f32::from_str(rpm).or(Err(()))?;
                    Self::Tempo(rpm)
                }
                _ => return Err(()),
            });
        }
        Err(())
    }
}
