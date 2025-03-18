use std::sync::{
    Weak,
    atomic::{AtomicBool, Ordering},
};

use atomic_float::AtomicF32;
use atomic_refcell::AtomicRefCell;
use eframe::{
    egui,
    egui::{Context, Event, RichText, Ui},
    epaint::Hsva,
};
use egui_plot::{Bar, BarChart, Legend, PlotResponse, PlotUi};
use errors::{LogErrorWithExt, LogOptionWithExt, minitrace};
use log::error;
use num_traits::identities::Zero;
use sync::Mutex;

use crate::{BPMDetectionParameters, BUILD_PROFILE, BUILD_TIME, egui::Color32, gui_remote::HistogramDataPoints};

pub struct BPMDetectionGUI<P: BPMDetectionParameters + 'static> {
    // keys_sender, gui_exit_callback and buffer_redraw belong to the GUI Remote,
    // that ultimately is held by the main app, which can drop it to let know the GUI app that we are exiting
    pub(crate) keys_sender: Weak<Mutex<Option<Box<dyn FnMut(&'static str) + Send>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) on_gui_exit_callback: Weak<Mutex<Option<Box<dyn Fn() + Send>>>>,
    pub live_parameters: P,
    pub(crate) histogram_data_points: Weak<AtomicRefCell<HistogramDataPoints>>,
    pub(crate) interpolated_data_points: Vec<f32>,
    pub(crate) estimated_bpm: Weak<AtomicF32>,
    pub(crate) daw_bpm: Weak<AtomicF32>,
    pub(crate) should_save: Weak<AtomicBool>,
}

#[allow(forbidden_lint_groups)]
#[allow(clippy::too_many_arguments)]
impl<P: BPMDetectionParameters> BPMDetectionGUI<P> {
    #[minitrace::trace]
    fn attach_barchart(&mut self, plot_ui: &mut PlotUi) -> Option<bool> {
        let histogram_data_points = self
            .histogram_data_points
            .upgrade()
            .log_error_msg("histogram_data_points weak reference is gone, leaving")?;
        let histogram_data_points = histogram_data_points
            .try_borrow()
            .log_error_msg("race condition while acquiring histogram_data_points, skipping frame")
            .ok()?;

        // so we interpolate based on normalized data
        let max_y = histogram_data_points.inbound_histogram_data_points.iter().max_by(|x, y| x.total_cmp(y))?;
        if max_y.is_zero() {
            return None;
        }

        if self.interpolated_data_points.len() != histogram_data_points.inbound_histogram_data_points.len() {
            self.interpolated_data_points.resize(0, 0.0);
            self.interpolated_data_points.resize(histogram_data_points.inbound_histogram_data_points.len(), 0.0);
            for (x, y) in histogram_data_points.inbound_histogram_data_points.iter().enumerate() {
                self.interpolated_data_points[x] = *y / max_y;
            }
        }

        let elapsed = histogram_data_points.inbound_histogram_data_update.elapsed();
        let interpolation_duration = self.live_parameters.get_gui_config().interpolation_duration;
        let interpolation_ratio = (elapsed.as_micros() as f32 / interpolation_duration.as_micros() as f32).min(1.0);
        let interpolation_ratio =
            interpolation_ratio.powf(1.0 / self.live_parameters.get_gui_config().interpolation_curve);

        for (y, interpolated_y) in
            histogram_data_points.inbound_histogram_data_points.iter().zip(self.interpolated_data_points.iter_mut())
        {
            *interpolated_y = y / max_y * interpolation_ratio + *interpolated_y * (1.0 - interpolation_ratio);
        }

        // so max is always 1 after interpolation, otherwise the y axis will be jumpy
        let max_interpolated_y = self.interpolated_data_points.iter().max_by(|x, y| x.total_cmp(y))?;

        let live_parameters = self.live_parameters.get_static_bpm_detection_parameters();
        let min_x = live_parameters.index_to_bpm(0);
        let max_x = live_parameters.index_to_bpm(histogram_data_points.inbound_histogram_data_points.len());

        drop(histogram_data_points);

        let mut prev = f64::from(self.live_parameters.get_static_bpm_detection_parameters().index_to_bpm(1));

        plot_ui.bar_chart(BarChart::new(
            (self.interpolated_data_points.iter().enumerate().map(|(x, y)| {
                let y = f64::from(*y / max_interpolated_y);
                let x = f64::from(self.live_parameters.get_static_bpm_detection_parameters().index_to_bpm(x));

                let width = ((x - prev) * 1.5).abs();
                prev = x;

                Bar::new(x, y)
                    .fill(Hsva { h: (x as f32 - min_x) / (max_x - min_x), s: 0.5 + y as f32 / 2.0, v: 0.5, a: 1.0 })
                    .width(width)
            }))
            .chain(
                [
                    Bar::new(
                        parameter::Asf64::get(&self.live_parameters.get_static_bpm_detection_parameters().lowest_bpm()),
                        0.0,
                    )
                    .width(0.0)
                    .fill(Color32::TRANSPARENT),
                    Bar::new(
                        parameter::Asf64::get(
                            &self.live_parameters.get_static_bpm_detection_parameters().highest_bpm(),
                        ),
                        0.0,
                    )
                    .width(0.0)
                    .fill(Color32::TRANSPARENT),
                ]
                .into_iter(),
            )
            .collect::<Vec<_>>(),
        ));
        Some(interpolation_ratio < 1.0)
    }

    #[minitrace::trace]
    fn draw_histogram(&mut self, ui: &mut Ui) -> PlotResponse<bool> {
        egui_plot::Plot::new("BPMs")
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .legend(Legend::default())
            .show(ui, |plot_ui| self.attach_barchart(plot_ui).unwrap_or_default())
    }
}

pub struct UpdateError;

impl<P: BPMDetectionParameters> BPMDetectionGUI<P> {
    pub fn update(&mut self, ctx: &Context) -> Result<(), UpdateError> {
        let (Some(estimated_bpm), Some(daw_bpm), Some(should_save)) =
            (self.estimated_bpm.upgrade(), self.daw_bpm.upgrade(), self.should_save.upgrade())
        else {
            error!("shared data weak references are gone");
            return Err(UpdateError);
        };

        if should_save.swap(false, Ordering::Relaxed) {
            self.live_parameters.save();
        }

        let Some(sender) = self.keys_sender.upgrade().log_info_msg("key sender weak ref is gone") else {
            return Err(UpdateError);
        };

        if let Some(sender) = sender.lock().as_mut() {
            ctx.input(|input| {
                for events in &input.events {
                    if let Event::Key { key, modifiers: _, pressed: true, .. } = events {
                        sender(key.name());
                    }
                }
            });
        }

        let refresh = egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        Self::legend(&estimated_bpm, &daw_bpm, ui);
                        ui.add_space(20.0);
                        self.settings_panel(ui);

                        let available_size = ui.available_size();
                        ui.add_space(available_size.y - ui.spacing().interact_size.y);

                        ui.horizontal(|ui| {
                            ui.label(BUILD_TIME);
                            ui.label(BUILD_PROFILE);
                        });
                    });
                    self.draw_histogram(ui).inner
                })
                .inner
            })
            .inner;
        if refresh {
            ctx.request_repaint();
        }
        Ok(())
    }
}

impl<P: BPMDetectionParameters> eframe::App for BPMDetectionGUI<P> {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.update(ctx).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_exit(&mut self) {
        let Some(on_gui_exit_callback) =
            self.on_gui_exit_callback.upgrade().log_error_msg("gui exit callback weakref is gone")
        else {
            return;
        };

        if let Some(on_gui_exit_callback) =
            on_gui_exit_callback.lock().as_ref().log_info_msg("gui exit callback not set")
        {
            on_gui_exit_callback();
        }
    }
}

impl<P: BPMDetectionParameters> BPMDetectionGUI<P> {
    fn legend(estimated_bpm: &AtomicF32, daw_bpm: &AtomicF32, ui: &mut Ui) {
        let to_text = |bpm: &AtomicF32| {
            let bpm = bpm.load(Ordering::Relaxed);
            if bpm.is_nan() { format!("{:>6.2}", "-") } else { format!("{bpm:>6.2}") }
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("DAW BPM      ").size(20.0).monospace());
                let bpm_text = to_text(daw_bpm);
                let bpm_text = RichText::new(bpm_text).size(20.0).monospace();
                ui.label(bpm_text);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Estimated BPM").size(20.0).monospace());
                let bpm_text = to_text(estimated_bpm);
                let bpm_text = RichText::new(bpm_text).size(20.0).monospace();
                ui.label(bpm_text);
            });
        });
    }
}
