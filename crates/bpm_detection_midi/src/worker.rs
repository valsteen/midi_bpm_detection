use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError},
    },
    thread,
    time::{Duration as StdDuration, Instant},
};

use bpm_detection_core::{
    BPMDetection,
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig, bpm_to_midi_clock_interval},
};
use errors::Result;
use log::error;
use sync::{ArcAtomicBool, Mutex};

use crate::{MidiServiceConfig, midi_output_trait::MidiOutput, worker_event::WorkerEvent};

const MAX_CLOCK_INTERVAL_MICROSECONDS: u64 = 1_000_000;
const FALLBACK_CLOCK_BPM: f32 = 120.0;
const CLOCK_BUSY_WAIT_MARGIN: StdDuration = StdDuration::from_millis(1);
const BPM_EVALUATION_DEBOUNCE: StdDuration = StdDuration::from_millis(50);
const MIDI_OUTPUT_IDLE_POLL_INTERVAL: StdDuration = StdDuration::from_millis(50);

/// Native desktop BPM worker.
///
/// This receives parsed worker events from `MidiIn`, updates `BPMDetection`, and publishes detected BPM/histogram
/// data back to the desktop UI. It does not own the virtual MIDI output directly; output is serialized through the
/// MIDI output thread so clock ticks, play/stop, and tempo SysEx share one owner.
pub struct Worker<B>
where
    B: BPMDetectionReceiver,
{
    bpm_detection_receiver: B,
    #[allow(forbidden_lint_groups)]
    #[allow(clippy::struct_field_names)]
    worker_events_receiver: Receiver<WorkerEvent>,
    midi_output_sender: Sender<MidiOutputCommand>,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    clock_interval_microseconds: Arc<AtomicU64>,
    send_tempo: ArcAtomicBool,
}

/// Commands owned by the native MIDI output thread.
///
/// These are not MIDI input events. They are side effects emitted by the desktop mode: transport messages, clock ticks
/// from the clock loop, or tempo feedback SysEx.
enum MidiOutputCommand {
    Play,
    Stop,
    Tempo(f32),
}

#[derive(Default)]
struct BpmEvaluationSchedule {
    scheduled_at: Option<Instant>,
    pending_static_config: Option<StaticBPMDetectionConfig>,
}

impl BpmEvaluationSchedule {
    fn wait_for(&self) -> Option<StdDuration> {
        self.scheduled_at.map(|scheduled_at| BPM_EVALUATION_DEBOUNCE.saturating_sub(scheduled_at.elapsed()))
    }

    fn is_due(&self) -> bool {
        self.scheduled_at.is_some_and(|scheduled_at| scheduled_at.elapsed() >= BPM_EVALUATION_DEBOUNCE)
    }

    fn schedule_evaluation(&mut self) {
        if self.scheduled_at.is_none() {
            self.scheduled_at = Some(Instant::now());
        }
    }

    fn schedule_static_update(&mut self, static_config: StaticBPMDetectionConfig) {
        self.pending_static_config = Some(static_config);
        self.schedule_evaluation();
    }

    fn complete_due_evaluation(&mut self) -> Option<StaticBPMDetectionConfig> {
        self.scheduled_at = None;
        self.pending_static_config.take()
    }
}

impl<B> Worker<B>
where
    B: BPMDetectionReceiver,
{
    #[allow(forbidden_lint_groups)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::too_many_lines)]
    fn worker_loop(&mut self, static_bpm_detection_config: StaticBPMDetectionConfig) {
        let mut bpm_detection = BPMDetection::new(static_bpm_detection_config);
        let mut bpm_evaluation_schedule = BpmEvaluationSchedule::default();

        loop {
            // Dynamic/static config edits are debounce points: wait briefly for related changes, then recompute once.
            let worker_event = if let Some(wait_for) = bpm_evaluation_schedule.wait_for() {
                match self.worker_events_receiver.recv_timeout(wait_for) {
                    Ok(worker_event) => Some(worker_event),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                let Ok(worker_event) = self.worker_events_receiver.recv() else {
                    break;
                };
                Some(worker_event)
            };

            let mut should_evaluate_bpm = false;

            if bpm_evaluation_schedule.is_due() {
                should_evaluate_bpm = true;
                if let Some(scheduled_bpm_detection_config) = bpm_evaluation_schedule.complete_due_evaluation() {
                    // Static config changes rebuild buffers/precomputed data, so apply them at the debounce boundary.
                    bpm_detection.update_static_config(scheduled_bpm_detection_config);
                }
            }

            if let Some(worker_event) = worker_event {
                // Consume all pending events, then compute BPM once for the whole batch.
                let mut worker_events_disconnected = false;
                should_evaluate_bpm |=
                    self.handle_worker_event(worker_event, &mut bpm_detection, &mut bpm_evaluation_schedule);
                loop {
                    match self.worker_events_receiver.try_recv() {
                        Ok(worker_event) => {
                            should_evaluate_bpm |= self.handle_worker_event(
                                worker_event,
                                &mut bpm_detection,
                                &mut bpm_evaluation_schedule,
                            );
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            worker_events_disconnected = true;
                            break;
                        }
                    }
                }
                if worker_events_disconnected && !should_evaluate_bpm {
                    break;
                }
            }

            if should_evaluate_bpm {
                let Some((histogram_data_points, bpm)) = bpm_detection.compute_bpm(&self.dynamic_bpm_detection_config)
                else {
                    continue;
                };

                // Detected BPM drives two desktop side effects: MIDI clock interval and optional tempo SysEx.
                self.clock_interval_microseconds.store(midi_clock_interval_microseconds(bpm), Ordering::Relaxed);
                if self.send_tempo.load(Ordering::Relaxed)
                    && let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Tempo(bpm))
                {
                    error!("could not send tempo to MIDI output thread : {err:?}");
                }

                self.bpm_detection_receiver.receive_bpm_histogram_data(histogram_data_points, bpm);
            }
        }
    }

    fn handle_worker_event(
        &mut self,
        worker_event: WorkerEvent,
        bpm_detection: &mut BPMDetection,
        bpm_evaluation_schedule: &mut BpmEvaluationSchedule,
    ) -> bool {
        match worker_event {
            WorkerEvent::TimedNoteOn(event) => {
                bpm_detection.receive_note_on(event);
                true
            }
            WorkerEvent::Play => {
                if let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Play) {
                    error!("could not send play to MIDI output thread : {err:?}");
                }
                false
            }
            WorkerEvent::Stop => {
                if let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Stop) {
                    error!("could not send stop to MIDI output thread : {err:?}");
                }
                false
            }
            WorkerEvent::DynamicBPMDetectionConfig(dynamic_bpm_detection_config) => {
                // Dynamic config changes scoring weights only; reuse the existing detection buffers.
                self.dynamic_bpm_detection_config = dynamic_bpm_detection_config;
                bpm_evaluation_schedule.schedule_evaluation();
                false
            }
            WorkerEvent::StaticBPMDetectionConfig(bpm_detection_config) => {
                // Static config changes detection model shape and is applied after the debounce delay.
                bpm_evaluation_schedule.schedule_static_update(bpm_detection_config);
                false
            }
        }
    }
}

pub fn spawn(
    midi_service_config: &MidiServiceConfig,
    static_bpm_detection_config: StaticBPMDetectionConfig,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    worker_receiver: Receiver<WorkerEvent>,
    midi_output: impl MidiOutput + Send + 'static,
    bpm_detection_receiver: impl BPMDetectionReceiver,
) -> Result<()> {
    let initial_clock_interval_microseconds = midi_clock_interval_microseconds(static_bpm_detection_config.bpm_center);
    let midi_output = Arc::new(Mutex::new(midi_output));
    let clock_interval_microseconds = Arc::new(AtomicU64::new(initial_clock_interval_microseconds));
    let midi_output_sender = spawn_midi_output_controller(
        midi_service_config.enable_midi_clock.clone(),
        clock_interval_microseconds.clone(),
        midi_output.clone(),
    )?;

    let mut worker = Worker {
        bpm_detection_receiver,
        worker_events_receiver: worker_receiver,
        midi_output_sender,
        dynamic_bpm_detection_config,
        clock_interval_microseconds,
        send_tempo: midi_service_config.send_tempo.clone(),
    };

    thread::Builder::new()
        .name("BPM worker".to_string())
        .spawn(move || worker.worker_loop(static_bpm_detection_config))?;
    Ok(())
}

fn spawn_midi_output_controller<C>(
    enable_midi_clock: ArcAtomicBool,
    clock_interval_microseconds: Arc<AtomicU64>,
    midi_output: Arc<Mutex<C>>,
) -> Result<Sender<MidiOutputCommand>>
where
    C: MidiOutput + Send + 'static,
{
    let (midi_output_sender, midi_output_receiver) = std::sync::mpsc::channel();

    let midi_output_thread = thread::Builder::new().name("MIDI output".to_string());

    midi_output_thread.spawn(move || {
        loop {
            if enable_midi_clock.load(Ordering::Relaxed) {
                if clock_emitter_loop(
                    &midi_output,
                    &midi_output_receiver,
                    &enable_midi_clock,
                    &clock_interval_microseconds,
                )
                .is_err()
                {
                    return;
                }
            } else {
                // Clock enable is an atomic flag, not a queued command, so poll while idle to react promptly.
                while !enable_midi_clock.load(Ordering::Relaxed) {
                    match midi_output_receiver.recv_timeout(MIDI_OUTPUT_IDLE_POLL_INTERVAL) {
                        Ok(command) => handle_midi_output_command(&midi_output, command),
                        Err(RecvTimeoutError::Disconnected) => return,
                        Err(RecvTimeoutError::Timeout) => (),
                    }
                }
            }
        }
    })?;

    Ok(midi_output_sender)
}

fn clock_emitter_loop<C>(
    clock_emitter: &Arc<Mutex<C>>,
    midi_output_receiver: &Receiver<MidiOutputCommand>,
    enable_midi_clock: &ArcAtomicBool,
    clock_interval_microseconds: &Arc<AtomicU64>,
) -> Result<(), ()>
where
    C: MidiOutput + Send + 'static,
{
    let mut next_tick = Instant::now();

    while enable_midi_clock.load(Ordering::Relaxed) {
        // Apply queued output commands before each tick; tempo commands are coalesced by the drain helper.
        drain_midi_output_commands(clock_emitter, midi_output_receiver)?;

        let interval = StdDuration::from_micros(sanitize_clock_interval_microseconds(
            clock_interval_microseconds.load(Ordering::Relaxed),
        ));

        // Calculate when the next tick should happen
        next_tick = schedule_next_tick(next_tick, Instant::now(), interval);

        // Sleep for the most part of the interval, leaving a small amount of time for busy-waiting
        if let Some(sleep_until) = next_tick.checked_sub(CLOCK_BUSY_WAIT_MARGIN) {
            while Instant::now() < sleep_until {
                thread::sleep(CLOCK_BUSY_WAIT_MARGIN);
            }
        }

        // Busy-waiting for fine-grained control
        while Instant::now() < next_tick {}

        // It's time to send the MIDI Timing Clock event
        clock_emitter.lock().tick(); // Replace with actual call to send MIDI event
    }
    Ok(())
}

fn drain_midi_output_commands<C>(
    midi_output: &Arc<Mutex<C>>,
    midi_output_receiver: &Receiver<MidiOutputCommand>,
) -> Result<(), ()>
where
    C: MidiOutput + Send + 'static,
{
    let mut latest_tempo = None;
    loop {
        match midi_output_receiver.try_recv() {
            // Tempo updates are state-like: when several are queued, only the newest one matters.
            Ok(MidiOutputCommand::Tempo(bpm)) => latest_tempo = Some(bpm),
            Ok(command) => handle_midi_output_command(midi_output, command),
            Err(TryRecvError::Disconnected) => return Err(()),
            Err(TryRecvError::Empty) => {
                if let Some(bpm) = latest_tempo {
                    handle_midi_output_command(midi_output, MidiOutputCommand::Tempo(bpm));
                }
                return Ok(());
            }
        }
    }
}

fn handle_midi_output_command<C>(midi_output: &Arc<Mutex<C>>, command: MidiOutputCommand)
where
    C: MidiOutput + Send + 'static,
{
    match command {
        MidiOutputCommand::Play => midi_output.lock().play(),
        MidiOutputCommand::Stop => midi_output.lock().stop(),
        MidiOutputCommand::Tempo(bpm) => midi_output.lock().sysex(&format!("TEMPO|{bpm}")),
    }
}

fn schedule_next_tick(previous_tick: Instant, now: Instant, interval: StdDuration) -> Instant {
    let scheduled_tick = previous_tick + interval;
    if scheduled_tick <= now { now + interval } else { scheduled_tick }
}

fn midi_clock_interval_microseconds(bpm: f32) -> u64 {
    if !bpm.is_finite() || bpm <= 0.0 {
        return fallback_clock_interval_microseconds();
    }
    let Some(interval) = bpm_to_midi_clock_interval(bpm).num_microseconds() else {
        return fallback_clock_interval_microseconds();
    };
    sanitize_clock_interval_microseconds(interval as u64)
}

fn sanitize_clock_interval_microseconds(interval_microseconds: u64) -> u64 {
    match interval_microseconds {
        0 => fallback_clock_interval_microseconds(),
        interval_microseconds => interval_microseconds.min(MAX_CLOCK_INTERVAL_MICROSECONDS),
    }
}

fn fallback_clock_interval_microseconds() -> u64 {
    bpm_to_midi_clock_interval(FALLBACK_CLOCK_BPM).num_microseconds().unwrap() as u64
}

#[cfg(test)]
mod tests {
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
        let midi_output = Arc::new(Mutex::new(TestMidiOutput::default()));
        let (sender, receiver) = std::sync::mpsc::channel();

        sender.send(MidiOutputCommand::Tempo(100.0)).unwrap();
        sender.send(MidiOutputCommand::Tempo(120.0)).unwrap();

        drain_midi_output_commands(&midi_output, &receiver).unwrap();

        assert_eq!(midi_output.lock().sysex_messages.as_slice(), ["TEMPO|120"]);
    }
}
