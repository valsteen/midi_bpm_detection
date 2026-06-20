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
    BPMDetection, NOTE_CAPACITY,
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

pub struct Worker<B, C>
where
    B: BPMDetectionReceiver,
    C: MidiOutput + Send + 'static,
{
    midi_output: Arc<Mutex<C>>,
    bpm_detection_receiver: B,
    #[allow(forbidden_lint_groups)]
    #[allow(clippy::struct_field_names)]
    worker_events_receiver: Receiver<WorkerEvent>,
    playback_sender: Sender<Playback>,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    clock_interval_microseconds: Arc<AtomicU64>,
    send_tempo: ArcAtomicBool,
}

enum Playback {
    Play,
    Stop,
}

impl<B, C> Worker<B, C>
where
    B: BPMDetectionReceiver,
    C: MidiOutput + Send + 'static,
{
    #[allow(forbidden_lint_groups)]
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::too_many_lines)]
    fn worker_loop(&mut self, static_bpm_detection_config: StaticBPMDetectionConfig) {
        let mut bpm_detection = BPMDetection::new(static_bpm_detection_config);
        let mut scheduled_bpm_detection_config_change: Option<StaticBPMDetectionConfig> = None;
        let mut schedule_evaluate_bpm: Option<Instant> = None;
        let mut buffered_events = Vec::with_capacity(NOTE_CAPACITY);

        loop {
            let worker_event = if let Some(schedule_evaluate_bpm) = &schedule_evaluate_bpm {
                let wait_for = StdDuration::from_millis(50).saturating_sub(schedule_evaluate_bpm.elapsed());
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

            let mut evaluate_bpm = false;

            if schedule_evaluate_bpm.is_some_and(|scheduled_at| scheduled_at.elapsed() > StdDuration::from_millis(50)) {
                schedule_evaluate_bpm = None;
                evaluate_bpm = true;
                if let Some(scheduled_bpm_detection_config) = scheduled_bpm_detection_config_change.take() {
                    bpm_detection.update_static_config(scheduled_bpm_detection_config);
                }
            }

            if let Some(worker_event) = worker_event {
                // consume all pending events, only compute bpm once we have all pending notes
                buffered_events.push(worker_event);
                buffered_events.extend(self.worker_events_receiver.try_iter());

                for worker_event in buffered_events.drain(..) {
                    match worker_event {
                        WorkerEvent::TimedNoteOn(event) => {
                            evaluate_bpm = true;
                            bpm_detection.receive_note_on(event);
                        }
                        WorkerEvent::TimingClock => {}
                        WorkerEvent::Play => {
                            if let Err(err) = self.playback_sender.send(Playback::Play) {
                                error!("could not send play to clock thread : {err:?}");
                            }
                        }
                        WorkerEvent::Stop => {
                            if let Err(err) = self.playback_sender.send(Playback::Stop) {
                                error!("could not send stop to clock thread : {err:?}");
                            }
                        }
                        WorkerEvent::DynamicBPMDetectionConfig(dynamic_bpm_detection_config) => {
                            self.dynamic_bpm_detection_config = dynamic_bpm_detection_config;
                            if schedule_evaluate_bpm.is_none() {
                                schedule_evaluate_bpm = Some(Instant::now());
                            }
                        }
                        WorkerEvent::StaticBPMDetectionConfig(bpm_detection_config) => {
                            scheduled_bpm_detection_config_change = Some(bpm_detection_config);
                            if schedule_evaluate_bpm.is_none() {
                                schedule_evaluate_bpm = Some(Instant::now());
                            }
                        }
                    }
                }
            }

            if evaluate_bpm {
                let Some((histogram_data_points, bpm)) = bpm_detection.compute_bpm(&self.dynamic_bpm_detection_config)
                else {
                    continue;
                };

                self.clock_interval_microseconds.store(midi_clock_interval_microseconds(bpm), Ordering::Relaxed);
                if self.send_tempo.load(Ordering::Relaxed) {
                    self.midi_output.lock().sysex(&format!("TEMPO|{bpm}"));
                }

                self.bpm_detection_receiver.receive_bpm_histogram_data(histogram_data_points, bpm);
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
    let playback_sender = spawn_playback_controller(
        midi_service_config.enable_midi_clock.clone(),
        clock_interval_microseconds.clone(),
        midi_output.clone(),
    )?;

    let mut worker = Worker {
        midi_output,
        bpm_detection_receiver,
        worker_events_receiver: worker_receiver,
        playback_sender,
        dynamic_bpm_detection_config,
        clock_interval_microseconds,
        send_tempo: midi_service_config.send_tempo.clone(),
    };

    thread::Builder::new()
        .name("BPM worker".to_string())
        .spawn(move || worker.worker_loop(static_bpm_detection_config))?;
    Ok(())
}

fn spawn_playback_controller<C>(
    enable_midi_clock: ArcAtomicBool,
    clock_interval_microseconds: Arc<AtomicU64>,
    midi_output: Arc<Mutex<C>>,
) -> Result<Sender<Playback>>
where
    C: MidiOutput + Send + 'static,
{
    let (playback_sender, playback_receiver) = std::sync::mpsc::channel();

    let midi_output_thread = thread::Builder::new().name("MIDI output".to_string());

    midi_output_thread.spawn(move || {
        loop {
            if enable_midi_clock.load(Ordering::Relaxed) {
                if clock_emitter_loop(
                    &midi_output,
                    &playback_receiver,
                    &enable_midi_clock,
                    &clock_interval_microseconds,
                )
                .is_err()
                {
                    return;
                }
            } else {
                while !enable_midi_clock.load(Ordering::Relaxed) {
                    match playback_receiver.recv_timeout(StdDuration::from_secs(1)) {
                        Ok(Playback::Play) => midi_output.lock().play(),
                        Ok(Playback::Stop) => midi_output.lock().stop(),
                        Err(RecvTimeoutError::Disconnected) => return,
                        Err(RecvTimeoutError::Timeout) => (),
                    }
                }
            }
        }
    })?;

    Ok(playback_sender)
}

fn clock_emitter_loop<C>(
    clock_emitter: &Arc<Mutex<C>>,
    playback: &Receiver<Playback>,
    enable_midi_clock: &ArcAtomicBool,
    clock_interval_microseconds: &Arc<AtomicU64>,
) -> Result<(), ()>
where
    C: MidiOutput + Send + 'static,
{
    let mut next_tick = Instant::now();

    while enable_midi_clock.load(Ordering::Relaxed) {
        match playback.try_recv() {
            Ok(Playback::Play) => clock_emitter.lock().play(),
            Ok(Playback::Stop) => clock_emitter.lock().stop(),
            Err(TryRecvError::Disconnected) => return Err(()),
            Err(TryRecvError::Empty) => {}
        }

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
    use super::*;

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
}
