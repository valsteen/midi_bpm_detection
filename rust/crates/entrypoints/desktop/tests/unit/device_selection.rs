use super::*;

fn virtual_port(name: &str) -> MidiInputPort {
    MidiInputPort::Virtual(name.to_string())
}

#[test]
fn refresh_displays_none_while_remembering_disappeared_selection() {
    let mut selection = DeviceSelection::new();
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);
    selection.select_index(1);

    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);

    assert_eq!(selection.selected(), &virtual_port("a"));
    assert_eq!(selection.displayed_selection(), Some(&MidiInputPort::None));
    assert_eq!(selection.selected_index(), Some(0));

    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);

    assert_eq!(selection.selected(), &virtual_port("a"));
    assert_eq!(selection.displayed_selection(), Some(&virtual_port("a")));
    assert_eq!(selection.selected_index(), Some(1));
}

#[test]
fn selecting_fallback_none_clears_disappeared_selection() {
    let mut selection = DeviceSelection::new();
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);
    selection.select_index(1);

    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("b")]);
    assert!(selection.displayed_selection_is_fallback());

    selection.select_index(0);
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);

    assert_eq!(selection.selected(), &MidiInputPort::None);
    assert_eq!(selection.displayed_selection(), Some(&MidiInputPort::None));
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
