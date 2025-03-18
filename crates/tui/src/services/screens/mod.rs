use errors::Result;
use strum::{EnumCount, IntoEnumIterator};

use crate::{
    action::Action,
    mode::Mode,
    services::Service,
    utils::dispatch::{ActionHandler, EventHandler},
};

#[derive(Default)]
pub struct Screens {
    current_mode: usize,
}

impl Service for Screens {}
impl EventHandler for Screens {}

impl ActionHandler for Screens {
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::Tick
            | Action::Render
            | Action::Resize(_, _)
            | Action::Suspend
            | Action::Quit
            | Action::Refresh
            | Action::Error(_)
            | Action::Down
            | Action::Up
            | Action::Help
            | Action::MIDIRestart
            | Action::TogglePlayback
            | Action::ToggleMidiClock
            | Action::ToggleSendTempo
            | Action::ShowGUI
            | Action::Save
            | Action::DynamicBPMDetectionConfig(_)
            | Action::StaticBPMDetectionConfig(_)
            | Action::SelectDevice(_) => Ok(None),
            Action::Switch(mode) => {
                self.current_mode = Mode::iter().position(|m| m == *mode).unwrap();
                Ok(None)
            }
            Action::NextScreen => {
                let n = (self.current_mode + 1) % Mode::COUNT;
                Ok(Some(Action::Switch(Mode::iter().nth(n).unwrap())))
            }
            Action::PrevScreen => {
                if self.current_mode == 0 {
                    Mode::COUNT - 1
                } else {
                    (self.current_mode - 1) % Mode::COUNT
                };
                Ok(Some(Action::Switch(Mode::iter().nth(self.current_mode).unwrap())))
            }
        }
    }
}
