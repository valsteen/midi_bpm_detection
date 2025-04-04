use bpm_detection_core::MidiInputPort;
use crossterm::event::MouseEvent;
use derivative::Derivative;
use errors::{Result, minitrace};
use itertools::{EitherOrBoth, Itertools};
use log::{error, info};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListDirection, ListState},
};

use crate::{
    action::Action,
    components::Component,
    config::Config,
    layout::{Position, centered_rect},
    mode::Mode,
    tui::{Event, Frame},
    utils::dispatch::{ActionHandler, EventHandler},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectDevice {
    #[derivative(Debug = "ignore")]
    devices: Vec<MidiInputPort>,
    active: bool,
    widget_state: ListState,
    #[derivative(Debug = "ignore")]
    selection: MidiInputPort,
    config: Option<Config>,
}

impl SelectDevice {
    #[must_use]
    pub fn box_new() -> Box<SelectDevice> {
        Box::new(Self {
            devices: vec![],
            active: false,
            widget_state: ListState::default().with_selected(Some(0)),
            selection: MidiInputPort::None,
            config: None,
        })
    }

    #[minitrace::trace]
    fn refresh_devices(&mut self, devices: &[MidiInputPort]) {
        let mut updated_selection = None;
        for diff in devices
            .iter()
            .enumerate()
            .merge_join_by(self.devices.iter().enumerate(), |(_, new_device), (_, old_device)| {
                new_device.cmp(old_device)
            })
        {
            match diff {
                EitherOrBoth::Both((new_index, new), (old_index, _)) => {
                    if new == &self.selection && new_index != old_index {
                        info!("updating from {old_index} to {new_index} because current selection moved in order");
                        updated_selection = Some(new_index);
                    }
                }
                EitherOrBoth::Left((i, device)) => {
                    if i > 0 && device == &self.selection {
                        info!("updating to {i} because device reappeared");
                        updated_selection = Some(i);
                    }
                    info!("Device added: {device}");
                }
                EitherOrBoth::Right((_old_index, device)) => {
                    if device == &self.selection {
                        info!("updating to 0 because device was removed");
                        updated_selection = Some(0);
                    }
                    info!("Device removed: {device}");
                }
            }
            if let Some(updated_selection) = updated_selection {
                info!("updated selection : {updated_selection}");
                self.widget_state.select(Some(updated_selection));
            }
        }
        self.devices = Vec::from(devices);
    }
}

impl Component for SelectDevice {
    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let default =
            self.config.as_ref().map_or(Style::default(), |config| config.styles[&Mode::DeviceView]["default"]);
        let devices = self.devices.iter().map(MidiInputPort::as_str);

        let list = List::new(devices)
            .block(Block::default().style(default).title("List").borders(Borders::ALL))
            .style(default)
            .highlight_style(default.add_modifier(Modifier::REVERSED))
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);

        let popup_area = centered_rect(rect, 50, Position::Start, 50, Position::Start);

        // TODO ideally, the widget should know and expose the position of each item
        // it knows only when drawing which is ok, because if it's not drawn, well, you have nothing to click on
        // with your mouse.
        f.render_stateful_widget(list, popup_area, &mut self.widget_state);

        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = Some(config);
        Ok(())
    }
}

impl ActionHandler for SelectDevice {
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>> {
        if let Action::Switch(mode) = action {
            self.active = mode == &Mode::DeviceView;
            return Ok(None);
        }

        if self.active {
            let selection = match action {
                Action::Up => match self.widget_state.selected() {
                    Some(0) | None => self.devices.len() - 1,
                    Some(s) => s - 1,
                },
                Action::Down => {
                    let Some(selection) =
                        (self.widget_state.selected().unwrap_or_default() + 1).checked_rem(self.devices.len())
                    else {
                        return Ok(None);
                    };
                    selection
                }
                _ => return Ok(None),
            };
            if let Some(device) = self.devices.get(selection) {
                self.widget_state.select(Some(selection));
                self.selection = device.clone();
                info!("selected device #{selection}");
                return Ok(Some(Action::SelectDevice(device.clone())));
            }
            error!("widget selected a device that is gone");
            return Ok(None);
        }

        Ok(None)
    }
}

impl EventHandler for SelectDevice {
    fn handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        if let Event::DeviceList(device_list) = event {
            self.refresh_devices(device_list);
        }
        self.default_handle_event(event)
    }

    fn handle_mouse_events(&mut self, _mouse: MouseEvent) -> Result<Option<Action>> {
        if self.active {
            // TODO handle mouse
            //info!("{} {}", mouse.column, mouse.row);
        }

        Ok(None)
    }
}
