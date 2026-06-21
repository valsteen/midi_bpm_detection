use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{
        DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, NormalDistributionConfigAccessor,
        StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor,
    },
};
use errors::LogErrorWithExt;
use gui::{BPMDetectionConfig, GUIConfigAccessor};
use parameter::OnOff;

use crate::{config::DesktopConfig, controller::DesktopController};

pub type StaticConfigCallback = Arc<dyn Fn(StaticBPMDetectionConfig) + Send + Sync>;
pub type DynamicConfigCallback = Arc<dyn Fn(DynamicBPMDetectionConfig) + Send + Sync>;
pub type DesktopControllerSlot<B> = Arc<sync::Mutex<Option<DesktopController<B>>>>;

pub struct DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub config: DesktopConfig,
    pub controller: DesktopControllerSlot<B>,
    pub on_static_config_changed: StaticConfigCallback,
    pub on_dynamic_config_changed: DynamicConfigCallback,
}

impl<B> DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub fn propagate_static_changes(&self) {
        (self.on_static_config_changed)(self.config.static_bpm_detection_config.clone());
    }

    pub fn propagate_dynamic_changes(&self) {
        (self.on_dynamic_config_changed)(self.config.dynamic_bpm_detection_config.clone());
    }
}

impl<B> NormalDistributionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn std_dev(&self) -> f64 {
        self.config.static_bpm_detection_config.normal_distribution.std_dev
    }

    fn factor(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.factor
    }

    fn cutoff(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.cutoff
    }

    fn resolution(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.resolution
    }

    fn set_std_dev(&mut self, val: f64) {
        self.config.static_bpm_detection_config.normal_distribution.std_dev = val;
        self.propagate_static_changes();
    }

    fn set_factor(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.factor = val;
        self.propagate_static_changes();
    }

    fn set_cutoff(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.cutoff = val;
        self.propagate_static_changes();
    }

    fn set_resolution(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.resolution = val;
        self.propagate_static_changes();
    }
}

impl<B> DynamicBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn beats_lookback(&self) -> u8 {
        self.config.dynamic_bpm_detection_config.beats_lookback
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.time_distance_weight
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.octave_distance_weight
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.multiplier_weight
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.subdivision_weight
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight
    }

    fn high_tempo_bias(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.high_tempo_bias
    }

    fn set_beats_lookback(&mut self, val: u8) {
        self.config.dynamic_bpm_detection_config.beats_lookback = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_time_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.time_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_octave_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.octave_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_multiplier_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.multiplier_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_subdivision_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.subdivision_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_high_tempo_bias(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.high_tempo_bias = val;
        self.propagate_dynamic_changes();
    }
}

impl<B> StaticBPMDetectionConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn bpm_center(&self) -> f32 {
        self.config.static_bpm_detection_config.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.config.static_bpm_detection_config.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.config.static_bpm_detection_config.sample_rate
    }

    fn index_to_bpm(&self, index: usize) -> f32 {
        self.config.static_bpm_detection_config.index_to_bpm(index)
    }

    fn highest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.highest_bpm()
    }

    fn lowest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.lowest_bpm()
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.config.static_bpm_detection_config.bpm_center = val;
        self.propagate_static_changes();
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.config.static_bpm_detection_config.bpm_range = val;
        self.propagate_static_changes();
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.config.static_bpm_detection_config.sample_rate = val;
        self.propagate_static_changes();
    }
}

impl<B> GUIConfigAccessor for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn interpolation_duration(&self) -> Duration {
        self.config.gui_config.interpolation_duration
    }

    fn interpolation_curve(&self) -> f32 {
        self.config.gui_config.interpolation_curve
    }

    fn set_interpolation_duration(&mut self, val: Duration) {
        self.config.gui_config.interpolation_duration = val;
        self.propagate_dynamic_changes();
    }

    fn set_interpolation_curve(&mut self, val: f32) {
        self.config.gui_config.interpolation_curve = val;
        self.propagate_dynamic_changes();
    }
}

impl<B> BPMDetectionConfig for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn get_send_tempo(&self) -> bool {
        self.config.midi.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.config.midi.send_tempo.store(enabled, Ordering::Relaxed);
    }

    fn save(&mut self) {
        self.config.save().log_error_msg("Could not save configuration").ok();
    }

    fn desktop_controls(&mut self, ui: &mut gui::eframe::egui::Ui) {
        let mut controller_slot = self.controller.lock();
        let Some(controller) = controller_slot.as_mut() else {
            ui.label("MIDI service is starting");
            return;
        };

        let devices = controller.device_selection().devices().to_vec();
        let selected = controller.device_selection().selected().clone();
        let mut selected_index = controller.device_selection().selected_index().unwrap_or_default();

        gui::eframe::egui::ComboBox::from_label("MIDI input").selected_text(selected.as_str()).show_ui(ui, |ui| {
            for (index, device) in devices.iter().enumerate() {
                ui.selectable_value(&mut selected_index, index, device.as_str());
            }
        });

        if Some(selected_index) != controller.device_selection().selected_index() {
            controller.select_device_index(selected_index).log_error_msg("Could not select MIDI input").ok();
        }

        if ui.button("Refresh MIDI inputs").clicked() {
            controller.refresh_devices().log_error_msg("Could not refresh MIDI input list").ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex as StdMutex},
        time::Duration,
    };

    use bpm_detection_core::{
        bpm_detection_receiver::BPMDetectionReceiver,
        parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor},
    };
    use bpm_detection_midi::MidiServiceConfig;
    use gui::{GUIConfig, GUIConfigAccessor};
    use sync::ArcAtomicBool;

    use super::*;
    use crate::config::{AppConfig, DesktopConfig};

    #[derive(Clone)]
    struct TestReceiver;

    impl BPMDetectionReceiver for TestReceiver {
        fn receive_bpm_histogram_data(&mut self, _histogram_data_points: &[f32], _detected_bpm: f32) {}

        fn receive_daw_bpm(&self, _bpm: f32) {}
    }

    fn desktop_config() -> DesktopConfig {
        DesktopConfig {
            app: AppConfig::default(),
            gui_config: GUIConfig::default(),
            midi: MidiServiceConfig {
                device_name: "Desktop".to_string(),
                send_tempo: ArcAtomicBool::new(false),
                enable_midi_clock: ArcAtomicBool::new(false),
            },
            static_bpm_detection_config: StaticBPMDetectionConfig::default(),
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig::default(),
        }
    }

    fn base_config(
        static_changes: Arc<StdMutex<Vec<StaticBPMDetectionConfig>>>,
        dynamic_changes: Arc<StdMutex<Vec<DynamicBPMDetectionConfig>>>,
    ) -> DesktopBaseConfig<TestReceiver> {
        DesktopBaseConfig {
            config: desktop_config(),
            controller: Arc::new(sync::Mutex::new(None)),
            on_static_config_changed: Arc::new(move |config| {
                static_changes.lock().expect("static changes lock should not be poisoned").push(config);
            }),
            on_dynamic_config_changed: Arc::new(move |config| {
                dynamic_changes.lock().expect("dynamic changes lock should not be poisoned").push(config);
            }),
        }
    }

    #[test]
    fn static_parameter_setter_propagates_static_config() {
        let static_changes = Arc::new(StdMutex::new(Vec::new()));
        let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
        let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

        config.set_bpm_center(120.0);

        let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
        let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
        assert_eq!(static_changes.len(), 1);
        assert!((static_changes[0].bpm_center - 120.0).abs() < f32::EPSILON);
        assert!(dynamic_changes.is_empty());
    }

    #[test]
    fn gui_parameter_setter_propagates_dynamic_config() {
        let static_changes = Arc::new(StdMutex::new(Vec::new()));
        let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
        let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

        config.set_interpolation_duration(Duration::from_millis(250));

        let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
        let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
        assert!(static_changes.is_empty());
        assert_eq!(dynamic_changes.len(), 1);
        assert_eq!(config.interpolation_duration(), Duration::from_millis(250));
    }
}
