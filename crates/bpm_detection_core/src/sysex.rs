use std::str::{FromStr, from_utf8};

use wmidi::MidiMessage;

pub enum SysExCommand {
    Tempo(f32),
    Play,
    Stop,
}

impl TryFrom<&MidiMessage<'_>> for SysExCommand {
    type Error = ();

    fn try_from(value: &MidiMessage) -> Result<Self, Self::Error> {
        let sysex = match value {
            MidiMessage::SysEx(sysex) => *sysex,
            MidiMessage::OwnedSysEx(sysex) => sysex.as_slice(),
            _ => return Err(()),
        };

        let bytes = sysex.iter().map(|u7| u8::from(*u7)).collect::<Vec<_>>();
        let sysex_string = from_utf8(&bytes).or(Err(()))?;
        let mut parts = sysex_string.splitn(2, '|');
        Ok(match (parts.next(), parts.next()) {
            (Some("PLAY"), None) => Self::Play,
            (Some("STOP"), None) => Self::Stop,
            (Some("TEMPO"), Some(rpm)) => {
                let rpm = f32::from_str(rpm).or(Err(()))?;
                Self::Tempo(rpm)
            }
            _ => return Err(()),
        })
    }
}
