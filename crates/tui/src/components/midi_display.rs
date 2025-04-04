use std::collections::VecDeque;

use bpm_detection_core::StaticMidiMessage;
use derivative::Derivative;
use errors::{MakeReportExt, Result};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List},
};

use crate::{
    action::Action,
    components::Component,
    config::Config,
    layout::{Position, rect_x, rect_y},
    mode::Mode,
    tui::{Event, Frame},
    utils::dispatch::{ActionHandler, EventHandler},
};

const CAPACITY: usize = 50;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct MidiDisplay {
    active: bool,
    config: Option<Config>,
    received: VecDeque<String>,
    start_timestamp: u64,
}

impl Component for MidiDisplay {
    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        let zone = rect_y(rect_x(rect, 50, Position::End), 100, Position::Start);
        let list = List::new(self.received.iter().rev().take(zone.height as usize).rev().map(String::as_str))
            .style(self.config.as_ref().map_or(Style::default(), |config| config.styles[&Mode::DeviceView]["default"]))
            .block(Block::default().title("Notes").borders(Borders::ALL));
        f.render_widget(list, zone);

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }
}

impl EventHandler for MidiDisplay {
    fn handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        if let Event::Midi(midi_message) = event {
            if midi_message.midi_message == StaticMidiMessage::ActiveSensing
                || midi_message.midi_message == StaticMidiMessage::TimingClock
            {
                return Ok(None);
            }

            let text = if let StaticMidiMessage::OwnedSysEx(value) = &midi_message.midi_message {
                let bytes = value.iter().map(|u7| u8::from(*u7)).collect();
                let sysex_string = String::from_utf8(bytes).or(Err(()));
                let Ok(text) = sysex_string.report_msg("invalid sysex received") else {
                    return Ok(None);
                };
                text
            } else {
                format!("{midi_message:?}\n")
            };
            self.received.push_back(text);
            let exceed = self.received.len().saturating_sub(CAPACITY);
            self.received.drain(..exceed);
        }
        Ok(None)
    }
}

impl ActionHandler for MidiDisplay {
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>> {
        if let Action::Switch(mode) = action {
            self.active = mode == &Mode::DeviceView;
            return Ok(None);
        }
        Ok(None)
    }
}
