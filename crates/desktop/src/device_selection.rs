use bpm_detection_midi::MidiInputPort;

#[derive(Debug, Clone)]
pub struct DeviceSelection {
    devices: Vec<MidiInputPort>,
    selected: MidiInputPort,
    selected_index: Option<usize>,
}

impl Default for DeviceSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSelection {
    #[must_use]
    pub fn new() -> Self {
        Self { devices: Vec::new(), selected: MidiInputPort::None, selected_index: None }
    }

    pub fn refresh_devices(&mut self, mut devices: Vec<MidiInputPort>) {
        devices.sort_unstable_by(|left, right| left.sort_key().cmp(&right.sort_key()));

        let selected_index = devices.iter().position(|device| device == &self.selected);
        if let Some(index) = selected_index {
            self.selected_index = Some(index);
        } else {
            self.selected = MidiInputPort::None;
            self.selected_index = devices.iter().position(|device| device == &MidiInputPort::None);
        }

        self.devices = devices;
    }

    pub fn select_index(&mut self, index: usize) -> Option<MidiInputPort> {
        let device = self.devices.get(index)?.clone();
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
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn virtual_port(name: &str) -> MidiInputPort {
        MidiInputPort::Virtual(name.to_string())
    }

    #[test]
    fn refresh_selects_none_when_current_device_disappears() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);
        selection.select_index(1);

        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);

        assert_eq!(selection.selected(), &MidiInputPort::None);
        assert_eq!(selection.selected_index(), Some(0));
    }

    #[test]
    fn refresh_keeps_selected_device_when_it_moves() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);
        selection.select_index(1);

        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);

        assert_eq!(selection.selected(), &virtual_port("b"));
        assert_eq!(selection.selected_index(), Some(2));
    }

    #[test]
    fn select_index_returns_selected_device() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);

        let selected = selection.select_index(1);

        assert_eq!(selected, Some(virtual_port("a")));
        assert_eq!(selection.selected_index(), Some(1));
    }
}
