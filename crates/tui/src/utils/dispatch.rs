use crossterm::event::{KeyEvent, MouseEvent};
use errors::{Report, Result};
use futures::{StreamExt, TryStreamExt};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, tui::Event};

pub trait ActionHandler {
    /// Update the state of the component based on a received action. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    // TODO refactor any place where I still have a dumb loop
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>>;
}

pub trait EventHandler {
    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    ///
    ///
    fn handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        self.default_handle_event(event)
    }

    fn default_handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        let r = match event {
            Event::Key(key_event) => self.handle_key_events(key_event)?,
            Event::Mouse(mouse_event) => self.handle_mouse_events(*mouse_event)?,
            _ => None,
        };
        Ok(r)
    }
    /// Handle key events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `key` - A key event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    #[allow(unused_variables)]
    fn handle_key_events(&mut self, key: &KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }
    /// Handle mouse events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `mouse` - A mouse event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    #[allow(unused_variables)]
    fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }
}

pub async fn try_dispatch_concurrently<'a, H, E, F, I>(
    handlers: I,
    event: &E,
    action_tx: &UnboundedSender<Action>,
    f: F,
) -> Result<(), Report>
where
    H: 'a + ?Sized,
    I: Iterator<Item = &'a mut H>,
    F: Fn(&'a mut H, &E) -> Result<Option<Action>, Report>,
    E: 'a,
{
    futures::stream::iter(handlers)
        .map(Ok)
        .try_for_each_concurrent(None, |component| async {
            if let Some(action) = f(component, event)? {
                action_tx.send(action)?;
            }
            Ok::<_, Report>(())
        })
        .await?;

    Ok(())
}
