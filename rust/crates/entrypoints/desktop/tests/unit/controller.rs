use errors::Report;

use super::*;

fn virtual_port(name: &str) -> MidiInputPort {
    MidiInputPort::Virtual(name.to_string())
}

#[test]
fn select_after_connecting_commits_selection_after_connect_succeeds() {
    let mut selection = DeviceSelection::new();
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);

    select_after_connecting(&mut selection, 1, |port| {
        assert_eq!(port, &virtual_port("a"));
        Ok(())
    })
    .expect("selection should succeed");

    assert_eq!(selection.selected(), &virtual_port("a"));
    assert_eq!(selection.selected_index(), Some(1));
}

#[test]
fn select_after_connecting_keeps_selection_after_connect_fails() {
    let mut selection = DeviceSelection::new();
    selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);
    selection.select_index(2).expect("initial selection should exist");

    let result = select_after_connecting(&mut selection, 1, |port| {
        assert_eq!(port, &virtual_port("a"));
        Err(Report::msg("connect failed"))
    });

    assert!(result.is_err());
    assert_eq!(selection.selected(), &virtual_port("b"));
    assert_eq!(selection.selected_index(), Some(2));
}
