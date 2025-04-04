use bpm_detection_core::{DynamicBPMDetectionParameters, MidiInputPort, StaticBPMDetectionParameters};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use strum::{Display, IntoStaticStr, VariantNames};

use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Eq, Display, VariantNames, IntoStaticStr)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Quit,
    Refresh,
    Error(String),
    Switch(Mode),
    NextScreen,
    PrevScreen,
    Help,
    Down,
    Up,
    MIDIRestart,
    SelectDevice(MidiInputPort),
    TogglePlayback,
    ToggleMidiClock,
    ShowGUI,
    DynamicBPMDetectionConfig(DynamicBPMDetectionParameters),
    StaticBPMDetectionConfig(StaticBPMDetectionParameters),
    Save,
    ToggleSendTempo,
}

impl Serialize for Action {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.into())
    }
}

impl<'a> TryFrom<&'a str> for Action {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, <Action as TryFrom<&'a str>>::Error> {
        Ok(match value {
            "Tick" => Action::Tick,
            "Render" => Action::Render,
            "Suspend" => Action::Suspend,
            "Quit" => Action::Quit,
            "Refresh" => Action::Refresh,
            "NextScreen" => Action::NextScreen,
            "PrevScreen" => Action::PrevScreen,
            "Help" => Action::Help,
            "Down" => Action::Down,
            "Up" => Action::Up,
            "TogglePlayback" => Action::TogglePlayback,
            "ToggleMidiClock" => Action::ToggleMidiClock,
            "ToggleSendTempo" => Action::ToggleSendTempo,
            "MIDIRestart" => Action::MIDIRestart,
            "ShowGUI" => Action::ShowGUI,
            "Save" => Action::Save,
            _ => return Err(value),
        })
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(Visitor)
    }
}

struct Visitor;

impl de::Visitor<'_> for Visitor {
    type Value = Action;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string representing a valid variant")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Self::Value::try_from(value).map_err(E::custom)
    }
}
