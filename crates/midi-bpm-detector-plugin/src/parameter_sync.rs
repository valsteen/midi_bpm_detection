use std::time::Duration as StdDuration;

use chrono::Duration;

pub const HOST_PARAMETER_SYNC_COALESCING_WINDOW: Duration = Duration::milliseconds(50);
pub const GUI_PARAMETER_SYNC_COALESCING_WINDOW: StdDuration = StdDuration::from_millis(200);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterSyncRequest {
    Host,
    Gui,
}
