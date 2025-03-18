use crossterm::{
    cursor,
    event::{DisableBracketedPaste, DisableFocusChange, DisableMouseCapture},
    terminal::LeaveAlternateScreen,
};

use crate::tui::io;

pub fn reset_crossterm() {
    if crossterm::terminal::is_raw_mode_enabled().unwrap_or_default() {
        crossterm::execute!(
            io(),
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen,
            DisableFocusChange,
            cursor::Show
        )
        .ok();

        crossterm::terminal::disable_raw_mode().ok();
    }
}
