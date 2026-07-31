use bpm_detection_config::{Settings, SettingsOwner};

#[derive(Clone, Debug)]
pub struct EditableSettings {
    pub bpm: Settings,
    pub send_tempo: Option<bool>,
}

impl SettingsOwner for EditableSettings {
    fn bpm_detection_settings(&self) -> &Settings {
        &self.bpm
    }

    fn bpm_detection_settings_mut(&mut self) -> &mut Settings {
        &mut self.bpm
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "these booleans are independent edit flags that may be set together; replacing them with an enum or \
              bitset would obscure the explicit commit map"
)]
pub struct GuiChanges {
    pub gui: bool,
    pub static_detection: bool,
    pub dynamic_detection: bool,
    pub send_tempo: bool,
}
