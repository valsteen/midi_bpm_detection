use wmidi::{Channel, ControlFunction, U7};

use super::*;

#[derive(Default)]
struct TestMidiOutput {
    sysex_messages: Vec<String>,
}

impl MidiOutput for TestMidiOutput {
    fn tick(&mut self) {}

    fn play(&mut self) {}

    fn stop(&mut self) {}

    fn cc(&mut self, _channel: Channel, _cc: ControlFunction, _value: U7) {}

    fn sysex(&mut self, value: &str) {
        self.sysex_messages.push(value.to_string());
    }
}

#[test]
fn sanitizes_missing_clock_interval_to_fallback_tempo() {
    assert_eq!(sanitize_clock_interval_microseconds(0), fallback_clock_interval_microseconds());
}

#[test]
fn caps_unusually_slow_clock_interval() {
    assert_eq!(sanitize_clock_interval_microseconds(2_000_000), MAX_CLOCK_INTERVAL_MICROSECONDS);
}

#[test]
fn schedules_next_tick_from_previous_tick_when_on_time() {
    let previous_tick = Instant::now();
    let interval = StdDuration::from_millis(20);

    assert_eq!(schedule_next_tick(previous_tick, previous_tick, interval), previous_tick + interval);
}

#[test]
fn schedules_next_tick_from_now_when_already_late() {
    let previous_tick = Instant::now();
    let interval = StdDuration::from_millis(20);
    let now = previous_tick + StdDuration::from_millis(30);

    assert_eq!(schedule_next_tick(previous_tick, now, interval), now + interval);
}

#[test]
fn coalesces_pending_tempo_commands() {
    let mut midi_output = TestMidiOutput::default();
    let (sender, receiver) = std::sync::mpsc::channel();

    sender.send(MidiOutputCommand::Tempo(100.0)).unwrap();
    sender.send(MidiOutputCommand::Tempo(120.0)).unwrap();

    drain_midi_output_commands(&mut midi_output, &receiver).unwrap();

    assert_eq!(midi_output.sysex_messages.as_slice(), ["TEMPO|120"]);
}
