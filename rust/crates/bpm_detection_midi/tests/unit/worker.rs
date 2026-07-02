use bpm_detection_core::{TimedNoteOn, note_events::NoteOn};
use chrono::Duration;
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

fn timed_note_on_at(timestamp: Duration) -> TimedNoteOn {
    TimedNoteOn { timestamp, event: NoteOn { channel: 0, pitch: 60, velocity: 100 } }
}

#[test]
fn command_intake_step_stops_when_commands_disconnect_before_work() {
    let (worker_commands_sender, worker_commands_receiver) = std::sync::mpsc::channel();
    let (midi_output_sender, _midi_output_receiver) = std::sync::mpsc::channel();
    let static_config = StaticBPMDetectionConfig::default();
    let mut command_intake = WorkerCommandIntake::new(
        worker_commands_receiver,
        midi_output_sender,
        static_config,
        DynamicBPMDetectionConfig::default(),
    );

    drop(worker_commands_sender);

    assert_eq!(command_intake.next_step(), WorkerLoopStep::Stop);
}

#[test]
fn command_intake_step_evaluates_drained_note_batch_even_after_disconnect() {
    let (worker_commands_sender, worker_commands_receiver) = std::sync::mpsc::channel();
    let (midi_output_sender, midi_output_receiver) = std::sync::mpsc::channel();
    let static_config = StaticBPMDetectionConfig::default();
    let mut command_intake = WorkerCommandIntake::new(
        worker_commands_receiver,
        midi_output_sender,
        static_config,
        DynamicBPMDetectionConfig::default(),
    );

    worker_commands_sender.send(BpmWorkerCommand::Play).unwrap();
    worker_commands_sender.send(BpmWorkerCommand::TimedNoteOn(timed_note_on_at(Duration::milliseconds(500)))).unwrap();
    drop(worker_commands_sender);

    assert_eq!(command_intake.next_step(), WorkerLoopStep::EvaluateBpm);
    assert!(matches!(midi_output_receiver.try_recv(), Ok(MidiOutputCommand::Play)));
}

#[test]
fn command_intake_step_applies_due_static_config_at_debounce_boundary() {
    let (_worker_commands_sender, worker_commands_receiver) = std::sync::mpsc::channel();
    let (midi_output_sender, _midi_output_receiver) = std::sync::mpsc::channel();
    let static_config = StaticBPMDetectionConfig::default();
    let mut command_intake = WorkerCommandIntake::new(
        worker_commands_receiver,
        midi_output_sender,
        static_config.clone(),
        DynamicBPMDetectionConfig::default(),
    );

    command_intake.bpm_evaluation_schedule.schedule_static_update(StaticBPMDetectionConfig {
        sample_rate: static_config.sample_rate + 1,
        ..static_config
    });
    command_intake.bpm_evaluation_schedule.scheduled_at = Instant::now().checked_sub(BPM_EVALUATION_DEBOUNCE);

    assert_eq!(command_intake.next_step(), WorkerLoopStep::EvaluateBpm);
    assert!(command_intake.bpm_evaluation_schedule.pending_static_config.is_none());
    assert!(command_intake.bpm_evaluation_schedule.scheduled_at.is_none());
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
