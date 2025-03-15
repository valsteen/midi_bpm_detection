#![cfg(target_arch = "wasm32")]
#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]

use crate::{LiveConfig, QueueItem};
use atomic_refcell::AtomicRefCell;
use chrono::Duration;
use errors::{LogErrorWithExt, Result};
use futures::{channel::mpsc::Sender, StreamExt};
use gui::{create_gui, start_gui, GuiRemote};
use instant::Instant;
use midi::{
    bpm_detection_receiver::BPMDetectionReceiver, midi_messages::MidiNoteOn, BPMDetection,
    DynamicBPMDetectionParameters, StaticBPMDetectionParameters, TimedTypedMidiMessage,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration as StdDuration,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::{js_sys::Promise, JsFuture};

async fn sleep(duration: StdDuration) {
    let promise = Promise::new(&mut |yes, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, i32::try_from(duration.as_millis()).unwrap())
            .unwrap();
    });
    JsFuture::from(promise).await.ok();
}

#[wasm_bindgen]
pub struct GuiRemoteWrapper {
    #[allow(dead_code)]
    gui_remote: GuiRemote, // javascript will hold this value, or the GUI will be dropped
    redraw_sender: Sender<QueueItem>,
}

#[wasm_bindgen]
impl GuiRemoteWrapper {
    pub fn event_in(&mut self, channel: u8, note: u8, velocity: u8, timestamp: f64) {
        let note = TimedTypedMidiMessage {
            timestamp: Duration::milliseconds(timestamp as i64),
            midi_message: MidiNoteOn { channel, note, velocity },
        };

        self.redraw_sender.try_send(QueueItem::Note(note)).log_error_msg("channel full").ok();
    }
}

const REDRAW_THRESHOLD_MILLIS: u64 = 200;

pub fn run() -> Result<GuiRemoteWrapper> {
    let (redraw_sender, mut redraw_receiver) = futures::channel::mpsc::channel(100);

    let live_config = LiveConfig::new(redraw_sender.clone());
    let static_bpm_detection_parameters = live_config.config.static_bpm_detection_parameters.clone();
    let mut dynamic_bpm_detection_parameters = live_config.config.dynamic_bpm_detection_parameters.clone();
    let (gui_remote, gui_builder) = create_gui(live_config);

    wasm_bindgen_futures::spawn_local({
        let mut gui_remote = gui_remote.clone();
        let update_static: Arc<AtomicRefCell<Option<StaticBPMDetectionParameters>>> =
            Arc::new(AtomicRefCell::default());
        let update_dynamic: Arc<AtomicRefCell<Option<DynamicBPMDetectionParameters>>> =
            Arc::new(AtomicRefCell::default());
        let update_notes: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let redraw_sender = redraw_sender.clone();

        async move {
            let mut bpm_detection = BPMDetection::new(static_bpm_detection_parameters);
            'main: while let Some(mut redraw_reason) = redraw_receiver.next().await {
                let now = Instant::now();
                loop {
                    match redraw_reason {
                        QueueItem::StaticParameters(new_static_bpm_detection_parameters) => {
                            let mut update = update_static.borrow_mut();

                            if update.is_none() {
                                wasm_bindgen_futures::spawn_local({
                                    let mut redraw_sender = redraw_sender.clone();
                                    async move {
                                        sleep(StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS)).await;
                                        redraw_sender.try_send(QueueItem::DelayedStaticUpdate).ok();
                                    }
                                });
                            }
                            *update = Some(new_static_bpm_detection_parameters);
                            continue 'main;
                        }
                        QueueItem::DynamicParameters(new_dynamic_bpm_detection_parameters) => {
                            let mut update = update_dynamic.borrow_mut();

                            if update.is_none() {
                                wasm_bindgen_futures::spawn_local({
                                    let mut redraw_sender = redraw_sender.clone();
                                    async move {
                                        sleep(StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS)).await;
                                        redraw_sender.try_send(QueueItem::DelayedDynamicUpdate).ok();
                                    }
                                });
                            }
                            *update = Some(new_dynamic_bpm_detection_parameters);
                            continue 'main;
                        }
                        QueueItem::Note(note) => {
                            bpm_detection.receive_midi_message(note);

                            if !update_notes.fetch_or(true, Ordering::Relaxed) {
                                wasm_bindgen_futures::spawn_local({
                                    let mut redraw_sender = redraw_sender.clone();
                                    async move {
                                        sleep(StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS)).await;
                                        redraw_sender.try_send(QueueItem::DelayedDynamicUpdate).ok();
                                    }
                                });
                            }
                            continue 'main;
                        }

                        QueueItem::DelayedStaticUpdate => {
                            if let Some(new_static_bpm_detection_parameters) = update_static.borrow_mut().take() {
                                bpm_detection.update_static_parameters(new_static_bpm_detection_parameters);
                            }
                        }
                        QueueItem::DelayedDynamicUpdate => {
                            update_notes.store(false, Ordering::Relaxed);
                            if let Some(new_dynamic_bpm_detection_parameters) = update_dynamic.borrow_mut().take() {
                                dynamic_bpm_detection_parameters = new_dynamic_bpm_detection_parameters;
                            }
                        }
                    }

                    if now.elapsed() > StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS) {
                        break;
                    }
                    let Ok(Some(next_redraw_reason)) = redraw_receiver.try_next() else {
                        break;
                    };
                    redraw_reason = next_redraw_reason;
                }

                let Some((histogram_data, bpm)) = bpm_detection.compute_bpm(&dynamic_bpm_detection_parameters) else {
                    continue;
                };

                gui_remote.receive_bpm_histogram_data(histogram_data, bpm);
            }
        }
    });

    start_gui(gui_builder)?;

    Ok(GuiRemoteWrapper { gui_remote, redraw_sender })
}
