#![allow(clippy::non_canonical_partial_ord_impl)]

use derivative::Derivative;
use std::fmt::Display;

const NO_SELECTION: &str = "<none selected>";

#[derive(Clone, PartialEq, Derivative)]
#[allow(clippy::non_canonical_partial_ord_impl)]
#[derivative(Eq, Debug, PartialOrd)]
#[derivative(Ord = "feature_allow_slow_enum", PartialOrd = "feature_allow_slow_enum")]
pub enum MidiInputPort {
    None,
    Virtual(String),
    Device(
        #[derivative(Ord = "ignore", Debug = "ignore")]
        #[derivative(PartialOrd = "ignore", Debug = "ignore")]
        midir::MidiInputPort,
        String,
    ),
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
}
