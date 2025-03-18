pub mod midi_display;
pub mod select_device;

use errors::Result;
use ratatui::layout::Rect;

use crate::{
    config::Config,
    tui::Frame,
    utils::dispatch::{ActionHandler, EventHandler},
};

/// `Component` is a trait that represents a visual and interactive element of the user interface.
/// Implementors of this trait can be registered with the main application loop and will be able to receive events,
/// update state, and be rendered on the screen.
pub trait Component: EventHandler + ActionHandler + Send + Sync {
    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;

    #[allow(unused_variables)]
    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        Ok(())
    }
}

pub trait ComponentNewBox<T>
where
    T: 'static + Default + Component,
{
    #[must_use]
    fn box_new() -> Box<dyn Component> {
        Box::<T>::default()
    }
}

impl<T> ComponentNewBox<T> for T where T: 'static + Component + Default {}
