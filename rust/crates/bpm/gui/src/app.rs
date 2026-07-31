use std::sync::{Arc, atomic::Ordering};

use bpm_detection_config::Settings;
use eframe::{
    egui::{RichText, Ui},
    epaint::Hsva,
};
use egui_plot::{Bar, BarChart, Legend, PlotResponse, PlotUi};
use errors::{LogErrorWithExt, minitrace};
use num_traits::identities::Zero;

use crate::{
    BUILD_PROFILE, BUILD_TIME, GuiChanges, display::DisplayState, editable_settings::EditableSettings, egui::Color32,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuiLifecycleOwner {
    ApplicationRuntime,
    ParentRuntime,
}

pub struct BPMDetectionGUI {
    pub(crate) display: Arc<DisplayState>,
    pub(crate) interpolated_data_points: Vec<f32>,
    prepared_estimated_bpm: f32,
    prepared_daw_bpm: f32,
}

impl BPMDetectionGUI {
    pub(crate) fn new(display: Arc<DisplayState>) -> Self {
        Self {
            display,
            interpolated_data_points: Vec::with_capacity(bpm_detection_config::max_histogram_data_buffer_size()),
            prepared_estimated_bpm: f32::NAN,
            prepared_daw_bpm: f32::NAN,
        }
    }

    pub fn attach_context(&mut self, context: &eframe::egui::Context, owner: GuiLifecycleOwner) {
        if owner == GuiLifecycleOwner::ParentRuntime {
            context.options_mut(|options| options.quit_shortcuts.clear());
        }
        self.display.context.borrow_mut().replace(context.clone());
    }

    pub fn prepare(&mut self) {
        self.prepared_estimated_bpm = self.display.estimated_bpm.load(Ordering::Relaxed);
        self.prepared_daw_bpm = self.display.daw_bpm.load(Ordering::Relaxed);
    }

    pub fn show(&mut self, ui: &mut Ui, settings: &mut EditableSettings) -> GuiChanges {
        let mut changes = GuiChanges::default();
        let refresh = ui
            .horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    self.legend(ui);
                    ui.add_space(20.0);
                    changes = Self::settings_panel(ui, settings);

                    let available_size = ui.available_size();
                    ui.add_space(available_size.y - ui.spacing().interact_size.y);

                    ui.horizontal(|ui| {
                        ui.label(BUILD_TIME);
                        ui.label(BUILD_PROFILE);
                    });
                });
                self.draw_histogram(ui, &settings.bpm).inner
            })
            .inner;
        if refresh {
            ui.ctx().request_repaint();
        }
        changes
    }

    #[minitrace::trace]
    fn attach_barchart(&mut self, config: &Settings, plot_ui: &mut PlotUi) -> Option<bool> {
        let histogram = self
            .display
            .histogram
            .try_borrow()
            .log_error_msg("GUI histogram snapshot busy; skipping this render frame")
            .ok()?;

        let max_y = histogram.data_points.iter().max_by(|x, y| x.total_cmp(y))?;
        if max_y.is_zero() {
            return None;
        }

        if self.interpolated_data_points.len() != histogram.data_points.len() {
            self.interpolated_data_points.resize(0, 0.0);
            self.interpolated_data_points.resize(histogram.data_points.len(), 0.0);
            for (x, y) in histogram.data_points.iter().enumerate() {
                self.interpolated_data_points[x] = *y / max_y;
            }
        }

        let elapsed = histogram.updated_at.elapsed();
        let interpolation_duration = config.gui_config.interpolation_duration;
        let interpolation_ratio = (elapsed.as_micros() as f32 / interpolation_duration.as_micros() as f32).min(1.0);
        let interpolation_ratio = interpolation_ratio.powf(1.0 / config.gui_config.interpolation_curve);

        for (y, interpolated_y) in histogram.data_points.iter().zip(self.interpolated_data_points.iter_mut()) {
            *interpolated_y = y / max_y * interpolation_ratio + *interpolated_y * (1.0 - interpolation_ratio);
        }

        let max_interpolated_y = self.interpolated_data_points.iter().max_by(|x, y| x.total_cmp(y))?;
        let static_config = &config.static_bpm_detection_config;
        let min_x = static_config.index_to_bpm(0);
        let max_x = static_config.index_to_bpm(histogram.data_points.len());
        drop(histogram);

        let mut prev = f64::from(static_config.index_to_bpm(1));
        plot_ui.bar_chart(BarChart::new(
            "BPM",
            self.interpolated_data_points
                .iter()
                .enumerate()
                .map(|(x, y)| {
                    let y = f64::from(*y / max_interpolated_y);
                    let x = f64::from(static_config.index_to_bpm(x));
                    let width = ((x - prev) * 1.5).abs();
                    prev = x;
                    Bar::new(x, y)
                        .fill(Hsva { h: (x as f32 - min_x) / (max_x - min_x), s: 0.5 + y as f32 / 2.0, v: 0.5, a: 1.0 })
                        .width(width)
                })
                .chain([
                    Bar::new(f64::from(static_config.lowest_bpm()), 0.0).width(0.0).fill(Color32::TRANSPARENT),
                    Bar::new(f64::from(static_config.highest_bpm()), 0.0).width(0.0).fill(Color32::TRANSPARENT),
                ])
                .collect::<Vec<_>>(),
        ));
        Some(interpolation_ratio < 1.0)
    }

    #[minitrace::trace]
    fn draw_histogram(&mut self, ui: &mut Ui, config: &Settings) -> PlotResponse<bool> {
        egui_plot::Plot::new("BPMs")
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .legend(Legend::default())
            .show(ui, |plot_ui| self.attach_barchart(config, plot_ui).unwrap_or_default())
    }

    fn legend(&self, ui: &mut Ui) {
        let to_text = |bpm: f32| {
            if bpm.is_nan() { format!("{:>6.2}", "-") } else { format!("{bpm:>6.2}") }
        };

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("DAW BPM      ").size(20.0).monospace());
                ui.label(RichText::new(to_text(self.prepared_daw_bpm)).size(20.0).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Estimated BPM").size(20.0).monospace());
                ui.label(RichText::new(to_text(self.prepared_estimated_bpm)).size(20.0).monospace());
            });
        });
    }
}
