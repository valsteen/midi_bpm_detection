use ratatui::prelude::*;

#[must_use]
pub fn centered_rect(r: Rect, percent_x: u16, position_x: Position, percent_y: u16, position_y: Position) -> Rect {
    rect_x(rect_y(r, percent_y, position_y), percent_x, position_x)
}

#[must_use]
pub fn rect(r: Rect, direction: Direction, percent: u16, position: Position) -> Rect {
    let (constraints, position) = position.get_constraints(percent);
    Layout::default().direction(direction).constraints(constraints).split(r)[position]
}

#[must_use]
pub fn rect_x(r: Rect, percent: u16, position: Position) -> Rect {
    rect(r, Direction::Horizontal, percent, position)
}

#[must_use]
pub fn rect_y(r: Rect, percent: u16, position: Position) -> Rect {
    rect(r, Direction::Vertical, percent, position)
}

// adjust as each 3 = % parameter
pub enum Position {
    Start,
    Middle,
    End,
}

impl Position {
    fn get_constraints(self, percent: u16) -> ([Constraint; 3], usize) {
        match self {
            Position::Start => {
                ([Constraint::Percentage(percent), Constraint::Percentage(100 - percent), Constraint::Percentage(0)], 0)
            }
            Position::Middle => (
                [
                    Constraint::Percentage((100 - percent) / 2),
                    Constraint::Percentage(percent),
                    Constraint::Percentage((100 - percent) / 2),
                ],
                1,
            ),
            Position::End => {
                ([Constraint::Percentage(0), Constraint::Percentage(100 - percent), Constraint::Percentage(percent)], 2)
            }
        }
    }
}
