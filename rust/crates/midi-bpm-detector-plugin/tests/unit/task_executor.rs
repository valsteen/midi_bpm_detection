use super::*;

#[test]
fn tempo_controller_frame_prefixes_big_endian_payload_length() {
    let frame = tempo_controller_frame(123.5);

    assert_eq!(u32::from_be_bytes(frame[..4].try_into().unwrap()), TEMPO_CONTROLLER_PAYLOAD_BYTES);
}

#[test]
fn tempo_controller_frame_writes_big_endian_bpm() {
    let frame = tempo_controller_frame(123.5);

    assert_eq!(frame[4..], 123.5f32.to_be_bytes());
}
