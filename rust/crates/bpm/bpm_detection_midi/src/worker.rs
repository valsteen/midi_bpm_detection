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
use sync::ArcAtomicBool;

use crate::{MidiServiceConfig, midi_output_trait::MidiOutput, worker_command::BpmWorkerCommand};

const MAX_CLOCK_INTERVAL_MICROSECONDS: u64 = 1_000_000;
const FALLBACK_CLOCK_BPM: f32 = 120.0;
const CLOCK_BUSY_WAIT_MARGIN: StdDuration = StdDuration::from_millis(1);
const BPM_EVALUATION_DEBOUNCE: StdDuration = StdDuration::from_millis(50);
const MIDI_OUTPUT_IDLE_POLL_INTERVAL: StdDuration = StdDuration::from_millis(50);

/// Commands owned by the native MIDI output thread.
///
/// These are not MIDI input events. They are side effects emitted by the desktop mode: transport messages, clock ticks
/// from the clock loop, or tempo feedback `SysEx`.
#[derive(Clone, Copy)]
enum MidiOutputCommand {
    Play,
    Stop,
    Tempo(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkerLoopStep {
    Continue,
    EvaluateBpm,
    Stop,
}

impl WorkerLoopStep {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::EvaluateBpm, _) | (_, Self::EvaluateBpm) => Self::EvaluateBpm,
            (Self::Stop, _) | (_, Self::Stop) => Self::Stop,
            (Self::Continue, Self::Continue) => Self::Continue,
        }
    }
}

enum WorkerCommandWait {
    Command(BpmWorkerCommand),
    Timeout,
    Disconnected,
}

#[derive(Default)]
struct BpmEvaluationSchedule {
    // The worker computes BPM at batch boundaries, not after every control/config message. This timestamp marks the
    // debounce window used to coalesce quick parameter edits and MIDI input bursts into one evaluation.
    scheduled_at: Option<Instant>,
    // Static config changes rebuild the detection model, so only the newest pending shape is applied when the
    // debounce window expires.
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

/// Runs the native desktop BPM worker.
///
/// This receives worker commands from `MidiIn`, updates `BPMDetection`, and publishes detected BPM/histogram data back
/// to the desktop UI. It does not own the virtual MIDI output directly; output is serialized through the MIDI output
/// thread so clock ticks, play/stop, and tempo `SysEx` share one owner.
fn run_worker_loop<B>(mut command_intake: WorkerCommandIntake, mut bpm_publisher: DetectedBpmPublisher<B>)
where
    B: BPMDetectionReceiver,
{
    loop {
        match command_intake.next_step() {
            WorkerLoopStep::Continue => (),
            WorkerLoopStep::EvaluateBpm => {
                if let Some((histogram_data_points, bpm)) = command_intake.compute_bpm() {
                    bpm_publisher.publish(histogram_data_points, bpm);
                }
            }
            WorkerLoopStep::Stop => break,
        }
    }
}

struct WorkerCommandIntake {
    commands_receiver: Receiver<BpmWorkerCommand>,
    midi_output_sender: Sender<MidiOutputCommand>,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    bpm_detection: BPMDetection,
    bpm_evaluation_schedule: BpmEvaluationSchedule,
}

impl WorkerCommandIntake {
    fn new(
        commands_receiver: Receiver<BpmWorkerCommand>,
        midi_output_sender: Sender<MidiOutputCommand>,
        static_bpm_detection_config: StaticBPMDetectionConfig,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    ) -> Self {
        Self {
            commands_receiver,
            midi_output_sender,
            dynamic_bpm_detection_config,
            bpm_detection: BPMDetection::new(static_bpm_detection_config),
            bpm_evaluation_schedule: BpmEvaluationSchedule::default(),
        }
    }

    fn next_step(&mut self) -> WorkerLoopStep {
        let worker_command = match self.wait_for_worker_command() {
            WorkerCommandWait::Command(worker_command) => Some(worker_command),
            WorkerCommandWait::Timeout => None,
            WorkerCommandWait::Disconnected => return WorkerLoopStep::Stop,
        };

        let mut worker_loop_step = self.complete_due_bpm_evaluation();

        if let Some(worker_command) = worker_command {
            worker_loop_step = worker_loop_step.merge(self.drain_worker_command_batch(worker_command));
        }

        worker_loop_step
    }

    fn compute_bpm(&mut self) -> Option<(&[f32], f32)> {
        self.bpm_detection.compute_bpm(&self.dynamic_bpm_detection_config)
    }

    fn wait_for_worker_command(&self) -> WorkerCommandWait {
        // Dynamic/static config edits are debounce points: wait briefly for related changes, then recompute once.
        let Some(wait_for) = self.bpm_evaluation_schedule.wait_for() else {
            return match self.commands_receiver.recv() {
                Ok(worker_command) => WorkerCommandWait::Command(worker_command),
                Err(_) => WorkerCommandWait::Disconnected,
            };
        };

        // This is not a polling loop. `recv_timeout` sleeps until either a worker command arrives or the scheduled BPM
        // evaluation becomes due.
        match self.commands_receiver.recv_timeout(wait_for) {
            Ok(worker_command) => WorkerCommandWait::Command(worker_command),
            Err(RecvTimeoutError::Timeout) => WorkerCommandWait::Timeout,
            Err(RecvTimeoutError::Disconnected) => WorkerCommandWait::Disconnected,
        }
    }

    fn complete_due_bpm_evaluation(&mut self) -> WorkerLoopStep {
        if !self.bpm_evaluation_schedule.is_due() {
            return WorkerLoopStep::Continue;
        }

        if let Some(scheduled_bpm_detection_config) = self.bpm_evaluation_schedule.complete_due_evaluation() {
            // Static config changes rebuild buffers/precomputed data, so apply them at the debounce boundary.
            self.bpm_detection.update_static_config(scheduled_bpm_detection_config);
        }
        WorkerLoopStep::EvaluateBpm
    }

    fn drain_worker_command_batch(&mut self, first_worker_command: BpmWorkerCommand) -> WorkerLoopStep {
        let mut worker_loop_step = self.handle_worker_command(first_worker_command);

        loop {
            match self.commands_receiver.try_recv() {
                Ok(worker_command) => {
                    worker_loop_step = worker_loop_step.merge(self.handle_worker_command(worker_command));
                }
                Err(TryRecvError::Empty) => return worker_loop_step,
                Err(TryRecvError::Disconnected) => {
                    return if worker_loop_step == WorkerLoopStep::EvaluateBpm {
                        WorkerLoopStep::EvaluateBpm
                    } else {
                        WorkerLoopStep::Stop
                    };
                }
            }
        }
    }

    fn handle_worker_command(&mut self, worker_command: BpmWorkerCommand) -> WorkerLoopStep {
        match worker_command {
            BpmWorkerCommand::TimedNoteOn(event) => {
                self.bpm_detection.receive_note_on(event);
                WorkerLoopStep::EvaluateBpm
            }
            BpmWorkerCommand::Play => {
                if let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Play) {
                    error!("could not send play to MIDI output thread : {err:?}");
                }
                WorkerLoopStep::Continue
            }
            BpmWorkerCommand::Stop => {
                if let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Stop) {
                    error!("could not send stop to MIDI output thread : {err:?}");
                }
                WorkerLoopStep::Continue
            }
            BpmWorkerCommand::DynamicBPMDetectionConfig(new_dynamic_bpm_detection_config) => {
                // Dynamic config changes scoring weights only; reuse the existing detection buffers.
                self.dynamic_bpm_detection_config = new_dynamic_bpm_detection_config;
                self.bpm_evaluation_schedule.schedule_evaluation();
                WorkerLoopStep::Continue
            }
            BpmWorkerCommand::StaticBPMDetectionConfig(bpm_detection_config) => {
                // Static config changes detection model shape and is applied after the debounce delay.
                self.bpm_evaluation_schedule.schedule_static_update(bpm_detection_config);
                WorkerLoopStep::Continue
            }
        }
    }
}

struct DetectedBpmPublisher<B>
where
    B: BPMDetectionReceiver,
{
    receiver: B,
    midi_output_sender: Sender<MidiOutputCommand>,
    clock_interval_microseconds: Arc<AtomicU64>,
    send_tempo: ArcAtomicBool,
}

impl<B> DetectedBpmPublisher<B>
where
    B: BPMDetectionReceiver,
{
    fn publish(&mut self, histogram_data_points: &[f32], bpm: f32) {
        // Detected BPM drives two desktop side effects: MIDI clock interval and optional tempo SysEx.
        self.clock_interval_microseconds.store(midi_clock_interval_microseconds(bpm), Ordering::Relaxed);
        if self.send_tempo.load(Ordering::Relaxed)
            && let Err(err) = self.midi_output_sender.send(MidiOutputCommand::Tempo(bpm))
        {
            error!("could not send tempo to MIDI output thread : {err:?}");
        }

        self.receiver.receive_bpm_histogram_data(histogram_data_points, bpm);
    }
}

pub fn spawn(
    midi_service_config: &MidiServiceConfig,
    static_bpm_detection_config: StaticBPMDetectionConfig,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    worker_commands_receiver: Receiver<BpmWorkerCommand>,
    midi_output: impl MidiOutput + Send + 'static,
    bpm_detection_receiver: impl BPMDetectionReceiver,
) -> Result<()> {
    let initial_clock_interval_microseconds = midi_clock_interval_microseconds(static_bpm_detection_config.bpm_center);
    let clock_interval_microseconds = Arc::new(AtomicU64::new(initial_clock_interval_microseconds));
    let midi_output_sender = spawn_midi_output_controller(
        midi_service_config.enable_midi_clock.clone(),
        clock_interval_microseconds.clone(),
        midi_output,
    )?;

    let worker_midi_output_sender = midi_output_sender.clone();
    let bpm_publisher = DetectedBpmPublisher {
        receiver: bpm_detection_receiver,
        midi_output_sender,
        clock_interval_microseconds,
        send_tempo: midi_service_config.send_tempo.clone(),
    };

    thread::Builder::new().name("BPM worker".to_string()).spawn(move || {
        // `WorkerCommandIntake::new` builds `BPMDetection`, whose fixed-capacity note buffer is large. Keep that
        // construction on the owning worker thread; constructing it on the MIDI service/main startup path can
        // overflow smaller thread stacks, especially in debug builds.
        let command_intake = WorkerCommandIntake::new(
            worker_commands_receiver,
            worker_midi_output_sender,
            static_bpm_detection_config,
            dynamic_bpm_detection_config,
        );
        run_worker_loop(command_intake, bpm_publisher);
    })?;
    Ok(())
}

fn spawn_midi_output_controller<C>(
    enable_midi_clock: ArcAtomicBool,
    clock_interval_microseconds: Arc<AtomicU64>,
    mut midi_output: C,
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
                    &mut midi_output,
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
                        Ok(command) => handle_midi_output_command(&mut midi_output, command),
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
    clock_emitter: &mut C,
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
        clock_emitter.tick();
    }
    Ok(())
}

fn drain_midi_output_commands<C>(
    midi_output: &mut C,
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

fn handle_midi_output_command<C>(midi_output: &mut C, command: MidiOutputCommand)
where
    C: MidiOutput + Send + 'static,
{
    match command {
        MidiOutputCommand::Play => midi_output.play(),
        MidiOutputCommand::Stop => midi_output.stop(),
        MidiOutputCommand::Tempo(bpm) => midi_output.sysex(&format!("TEMPO|{bpm}")),
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
    let Ok(interval) = u64::try_from(interval) else {
        return fallback_clock_interval_microseconds();
    };
    sanitize_clock_interval_microseconds(interval)
}

fn sanitize_clock_interval_microseconds(interval_microseconds: u64) -> u64 {
    match interval_microseconds {
        0 => fallback_clock_interval_microseconds(),
        interval_microseconds => interval_microseconds.min(MAX_CLOCK_INTERVAL_MICROSECONDS),
    }
}

fn fallback_clock_interval_microseconds() -> u64 {
    // 120 BPM does not map to an integer number of microseconds per MIDI clock pulse, so keep the same conversion and
    // truncation path used for detected tempos instead of hardcoding a rounded constant.
    u64::try_from(bpm_to_midi_clock_interval(FALLBACK_CLOCK_BPM).num_microseconds().unwrap())
        .expect("fallback MIDI clock interval should be positive")
}

#[cfg(test)]
#[path = "../tests/unit/worker.rs"]
mod tests;
