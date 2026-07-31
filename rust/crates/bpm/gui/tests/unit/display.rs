use bpm_detection_core::bpm_detection_receiver::BPMDetectionReceiver;

use crate::create_gui;

#[test]
fn display_publication_is_a_no_op_after_gui_drop() {
    let (mut publisher, context, gui) = create_gui();
    drop(gui);

    publisher.receive_bpm_histogram_data(&[0.25, 1.0, 0.5], 123.0);
    publisher.receive_daw_bpm(120.0);
    context.request_repaint();

    assert!(!context.egui_wants_keyboard_input());
}
