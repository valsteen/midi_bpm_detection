use bpm_detection_midi::MidiInputPort;

#[derive(Debug, Clone)]
pub struct DeviceSelection {
    devices: Vec<MidiInputPort>,
    selected: MidiInputPort,
    selected_index: Option<usize>,
    revision: u64,
}

impl Default for DeviceSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSelection {
    #[must_use]
    pub fn new() -> Self {
        Self { devices: Vec::new(), selected: MidiInputPort::None, selected_index: None, revision: 0 }
    }

    pub fn refresh_devices(&mut self, mut devices: Vec<MidiInputPort>) {
        devices.sort_unstable_by(|left, right| left.sort_key().cmp(&right.sort_key()));

        let selected_index = devices.iter().position(|device| device == &self.selected);
        let selected_index =
            selected_index.or_else(|| devices.iter().position(|device| device == &MidiInputPort::None));
        if self.devices != devices || self.selected_index != selected_index {
            self.revision = self.revision.wrapping_add(1);
        }
        self.selected_index = selected_index;
        self.devices = devices;
    }

    pub fn select_index(&mut self, index: usize) -> Option<MidiInputPort> {
        let device = self.devices.get(index)?.clone();
        if self.selected != device || self.selected_index != Some(index) {
            self.revision = self.revision.wrapping_add(1);
        }
        self.selected = device.clone();
        self.selected_index = Some(index);
        Some(device)
    }

    #[must_use]
    pub fn devices(&self) -> &[MidiInputPort] {
        &self.devices
    }

    #[must_use]
    pub fn selected(&self) -> &MidiInputPort {
        &self.selected
    }

    #[must_use]
    pub fn displayed_selection(&self) -> Option<&MidiInputPort> {
        self.selected_index.and_then(|index| self.devices.get(index))
    }

    #[must_use]
    pub fn displayed_selection_is_fallback(&self) -> bool {
        self.displayed_selection().is_some_and(|device| device != &self.selected)
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }
}

#[cfg(test)]
#[path = "../tests/unit/device_selection.rs"]
mod tests;
