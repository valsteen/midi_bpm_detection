use std::fmt::{Debug, Display};

const NO_SELECTION: &str = "<none selected>";

#[derive(Clone, PartialEq)]
pub enum MidiInputPort {
    None,
    Virtual(String),
    Device(midir::MidiInputPort, String),
}

impl Eq for MidiInputPort {}

impl Debug for MidiInputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.debug_tuple("None").finish(),
            Self::Virtual(name) => f.debug_tuple("Virtual").field(name).finish(),
            Self::Device(_, name) => f.debug_tuple("Device").field(name).finish(),
        }
    }
}

impl Display for MidiInputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl MidiInputPort {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            MidiInputPort::None => NO_SELECTION,
            MidiInputPort::Virtual(name) | MidiInputPort::Device(_, name) => name.as_str(),
        }
    }

    #[must_use]
    pub fn sort_key(&self) -> (u8, &str) {
        match self {
            MidiInputPort::None => (0, self.as_str()),
            MidiInputPort::Virtual(_) => (1, self.as_str()),
            MidiInputPort::Device(_, _) => (2, self.as_str()),
        }
    }
}
