use bpm_detection_midi::MidiInputPort;

use super::select_displayed_device;
use crate::device_selection::DeviceSelection;

fn virtual_port(name: &str) -> MidiInputPort {
    MidiInputPort::Virtual(name.to_string())
}

#[test]
fn confirming_displayed_fallback_returns_a_concrete_selection_action() {
    let mut selection = DeviceSelection::new();
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);
    selection.select_index(1);
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);

    let action = select_displayed_device(&mut selection, 0, true);

    assert_eq!(action, Some(0));
    assert_eq!(selection.selected(), &MidiInputPort::None);
}
