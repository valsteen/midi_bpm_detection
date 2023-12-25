use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, EnumIter};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter, EnumCount, Display)]
pub enum Mode {
    #[default]
    Home,
    DeviceView,
}
