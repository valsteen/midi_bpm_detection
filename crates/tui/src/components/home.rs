use errors::Result;

use ratatui::{prelude::*, widgets::Paragraph};

use super::Frame;
use crate::{
    action::Action,
    components::Component,
    config::Config,
    mode::Mode,
    utils::dispatch::{ActionHandler, EventHandler},
};

#[derive(Default)]
pub struct Home {
    config: Option<Config>,
    active: bool,
}

impl Component for Home {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        if self.active {
            f.render_widget(
                Paragraph::new("hello world").style(
                    self.config.as_ref().map_or(Style::default(), |config| config.styles[&Mode::Home]["default"]),
                ),
                area,
            );
        }

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }
}

impl EventHandler for Home {}

impl ActionHandler for Home {
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>> {
        if let Action::Switch(mode) = action {
            self.active = mode == &Mode::Home;
        }
        Ok(None)
    }
}
