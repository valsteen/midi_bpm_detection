#![cfg(target_arch = "wasm32")]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]

use std::time::Duration as StdDuration;

use bpm_detection_config::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use bpm_detection_core::{BPMDetection, TimedEvent, bpm_detection_receiver::BPMDetectionReceiver, note_events::NoteOn};
use chrono::Duration;
use errors::{LogErrorWithExt, Result};
use futures::{StreamExt, channel::mpsc::Sender};
use gui::{BpmDisplayPublisher, GuiContextHandle, GuiLifecycleOwner, create_gui, eframe};
use instant::Instant;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::{JsFuture, js_sys::Promise};

use crate::{QueueItem, WASMConfig, WasmApp};

async fn sleep(duration: StdDuration) {
    let promise = Promise::new(&mut |yes, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&yes, i32::try_from(duration.as_millis()).unwrap())
            .unwrap();
    });
    JsFuture::from(promise).await.ok();
}

pub(crate) fn keyboard_event_generates_tap(is_repeat: bool, egui_wants_keyboard_input: bool) -> bool {
    !is_repeat && !egui_wants_keyboard_input
}

#[wasm_bindgen]
pub struct GuiRemoteWrapper {
    gui_context: GuiContextHandle,
    redraw_sender: Sender<QueueItem>,
}

#[wasm_bindgen]
impl GuiRemoteWrapper {
    pub fn keyboard_event_in(&mut self, timestamp: f64, is_repeat: bool) {
        if !keyboard_event_generates_tap(is_repeat, self.gui_context.egui_wants_keyboard_input()) {
            return;
        }

        self.event_in(0, 0, 80, timestamp);
    }

    pub fn event_in(&mut self, channel: u8, note: u8, velocity: u8, timestamp: f64) {
        let note = TimedEvent {
            timestamp: Duration::milliseconds(timestamp as i64),
            event: NoteOn { channel, pitch: note, velocity },
        };

        self.redraw_sender.try_send(QueueItem::Note(note)).log_error_msg("channel full").ok();
    }
}

const REDRAW_THRESHOLD_MILLIS: u64 = 200;

fn schedule_delayed_static_update(mut redraw_sender: Sender<QueueItem>) {
    wasm_bindgen_futures::spawn_local(async move {
        sleep(StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS)).await;
        redraw_sender.try_send(QueueItem::DelayedStaticUpdate).ok();
    });
}

fn schedule_delayed_dynamic_update(mut redraw_sender: Sender<QueueItem>) {
    wasm_bindgen_futures::spawn_local(async move {
        sleep(StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS)).await;
        redraw_sender.try_send(QueueItem::DelayedDynamicUpdate).ok();
    });
}

pub fn run() -> Result<GuiRemoteWrapper> {
    let (redraw_sender, mut redraw_receiver) = futures::channel::mpsc::channel(100);

    let config = WASMConfig::default();
    let static_bpm_detection_config = config.bpm_detection.static_bpm_detection_config.clone();
    let mut dynamic_bpm_detection_config = config.bpm_detection.dynamic_bpm_detection_config.clone();
    let (publisher, gui_context, gui) = create_gui();
    let app = WasmApp::new(config, gui, redraw_sender.clone());

    wasm_bindgen_futures::spawn_local({
        let mut publisher: BpmDisplayPublisher = publisher;
        let redraw_sender = redraw_sender.clone();

        async move {
            let mut bpm_detection = BPMDetection::new(static_bpm_detection_config);
            let mut pending_static: Option<StaticBPMDetectionConfig> = None;
            let mut pending_dynamic: Option<DynamicBPMDetectionConfig> = None;
            let mut notes_changed = false;
            'main: while let Some(mut redraw_reason) = redraw_receiver.next().await {
                let now = Instant::now();
                loop {
                    match redraw_reason {
                        QueueItem::StaticParameters(new_static_bpm_detection_config) => {
                            if pending_static.is_none() {
                                schedule_delayed_static_update(redraw_sender.clone());
                            }
                            pending_static = Some(new_static_bpm_detection_config);
                            continue 'main;
                        }
                        QueueItem::DynamicParameters(new_dynamic_bpm_detection_config) => {
                            if pending_dynamic.is_none() {
                                schedule_delayed_dynamic_update(redraw_sender.clone());
                            }
                            pending_dynamic = Some(new_dynamic_bpm_detection_config);
                            continue 'main;
                        }
                        QueueItem::Note(note) => {
                            bpm_detection.receive_note_on(note);

                            if !notes_changed {
                                notes_changed = true;
                                schedule_delayed_dynamic_update(redraw_sender.clone());
                            }
                            continue 'main;
                        }

                        QueueItem::DelayedStaticUpdate => {
                            if let Some(new_static_bpm_detection_config) = pending_static.take() {
                                bpm_detection.update_static_config(new_static_bpm_detection_config);
                            }
                        }
                        QueueItem::DelayedDynamicUpdate => {
                            notes_changed = false;
                            if let Some(new_dynamic_bpm_detection_config) = pending_dynamic.take() {
                                dynamic_bpm_detection_config = new_dynamic_bpm_detection_config;
                            }
                        }
                    }

                    if now.elapsed() > StdDuration::from_millis(REDRAW_THRESHOLD_MILLIS) {
                        break;
                    }
                    let Ok(next_redraw_reason) = redraw_receiver.try_recv() else {
                        break;
                    };
                    redraw_reason = next_redraw_reason;
                }

                let Some((histogram_data, bpm)) = bpm_detection.compute_bpm(&dynamic_bpm_detection_config) else {
                    continue;
                };

                publisher.receive_bpm_histogram_data(histogram_data, bpm);
            }
        }
    });

    start_gui(app);

    Ok(GuiRemoteWrapper { gui_context, redraw_sender })
}

fn start_gui(mut app: WasmApp) {
    use eframe::wasm_bindgen::JsCast;

    let document = web_sys::window().expect("No window").document().expect("No document");
    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find the_canvas_id")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("the_canvas_id was not a HtmlCanvasElement");

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Dark);
                    app.gui.attach_context(&cc.egui_ctx, GuiLifecycleOwner::ParentRuntime);
                    Ok(Box::new(app))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}
