use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterVisitor, NormalDistributionConfig, StaticBPMDetectionConfig,
};
use gui::GUIConfig;
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Param, Params},
    prelude::{IntRange, RemoteControlsPage},
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use parameter_nih_plug::nih_plugin_parameter_group;
use sync::ArcAtomicOptionNonZeroU16;

use crate::{
    DeferredConfigUpdate,
    plugin_config::{PluginConfig, SendTempoOutputState},
    plugin_parameter_adapters::{
        PluginOnOffParam, to_plugin_duration_param, to_plugin_float_param, to_plugin_int_param, to_plugin_on_off_param,
    },
};

#[derive(Params)]
pub struct PluginGUIParams {
    #[id = "interpolation_duration"]
    pub interpolation_duration: FloatParam,
    #[id = "interpolation_curve"]
    pub interpolation_curve: FloatParam,
}

#[derive(Params)]
pub struct PluginDynamicParams {
    #[id = "beats_lookback"]
    pub beats_lookback: IntParam,
    #[nested]
    pub normal_distribution_weight: PluginOnOffParam,
    #[nested]
    pub time_distance_weight: PluginOnOffParam,
    #[nested]
    pub velocity_current_note_weight: PluginOnOffParam,
    #[nested]
    pub velocity_note_from_weight: PluginOnOffParam,
    #[nested]
    pub in_beat_range_weight: PluginOnOffParam,
    #[nested]
    pub multiplier_weight: PluginOnOffParam,
    #[nested]
    pub subdivision_weight: PluginOnOffParam,
    #[nested]
    pub octave_distance_weight: PluginOnOffParam,
    #[nested]
    pub pitch_distance_weight: PluginOnOffParam,
    #[nested]
    pub high_tempo_bias_weight: PluginOnOffParam,
}

#[nih_plugin_parameter_group(config = NormalDistributionConfig, group = "normal_distribution")]
pub struct NormalDistributionParams {
    pub std_dev: FloatParam,
    pub resolution: FloatParam,
    pub cutoff: FloatParam,
    pub factor: FloatParam,
}

#[nih_plugin_parameter_group(config = StaticBPMDetectionConfig, group = "StaticParams")]
pub struct PluginStaticParams {
    pub bpm_center: FloatParam,
    pub bpm_range: IntParam,
    #[nih_plugin_parameter(adapter = "float_u16_logarithmic")]
    pub sample_rate: FloatParam,
    #[nih_plugin_nested(group = "normal_distribution")]
    pub normal_distribution: NormalDistributionParams,
}

#[derive(Params)]
pub struct MidiBpmDetectorParams {
    pub editor_state: Arc<EguiState>,

    #[id = "send_tempo"]
    pub send_tempo: BoolParam,

    #[nested(group = "GUI")]
    pub gui_params: PluginGUIParams,
    #[nested(group = "StaticParams")]
    pub static_params: PluginStaticParams,
    #[nested(group = "DynamicParams")]
    pub dynamic_params: PluginDynamicParams,

    #[id = "daw_port"]
    pub daw_port: IntParam,
}

impl PluginDynamicParams {
    fn new(config: &DynamicBPMDetectionConfig, change_marker: &HostParameterChangeMarker) -> Self {
        let update_changed_at_f32 = change_marker.callback();
        let update_changed_at_u8 = change_marker.callback();
        let dynamic_parameters = DynamicBPMDetectionConfig::PARAMETERS;

        Self {
            beats_lookback: to_plugin_int_param(&dynamic_parameters.beats_lookback(), config, &update_changed_at_u8),
            normal_distribution_weight: to_plugin_on_off_param(
                &dynamic_parameters.normal_distribution_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            time_distance_weight: to_plugin_on_off_param(
                &dynamic_parameters.time_distance_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            velocity_current_note_weight: to_plugin_on_off_param(
                &dynamic_parameters.velocity_current_note_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            velocity_note_from_weight: to_plugin_on_off_param(
                &dynamic_parameters.velocity_note_from_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            in_beat_range_weight: to_plugin_on_off_param(
                &dynamic_parameters.in_beat_range_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            multiplier_weight: to_plugin_on_off_param(
                &dynamic_parameters.multiplier_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            subdivision_weight: to_plugin_on_off_param(
                &dynamic_parameters.subdivision_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            octave_distance_weight: to_plugin_on_off_param(
                &dynamic_parameters.octave_distance_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            pitch_distance_weight: to_plugin_on_off_param(
                &dynamic_parameters.pitch_distance_weight_field(),
                config,
                &update_changed_at_f32,
            ),
            high_tempo_bias_weight: to_plugin_on_off_param(
                &dynamic_parameters.high_tempo_bias_weight_field(),
                config,
                &update_changed_at_f32,
            ),
        }
    }

    pub(crate) fn add_remote_controls(&self, page: &mut impl RemoteControlsPage) {
        DynamicPluginParameterMapping::visit(self, DynamicRemoteControlParams { page });
    }

    pub(crate) fn read_dynamic_config(&self) -> DynamicBPMDetectionConfig {
        let mut config = DynamicBPMDetectionConfig::default();

        DynamicPluginParameterMapping::visit(self, DynamicHostConfigReader { config: &mut config });

        config
    }
}

impl PluginGUIParams {
    fn new(config: &GUIConfig, change_marker: &HostParameterChangeMarker) -> Self {
        let update_changed_at_f32 = change_marker.callback();
        let gui_parameters = GUIConfig::PARAMETERS;

        Self {
            interpolation_duration: to_plugin_duration_param(
                &gui_parameters.interpolation_duration(),
                config,
                &update_changed_at_f32,
            ),
            interpolation_curve: to_plugin_float_param(
                &gui_parameters.interpolation_curve(),
                config,
                &update_changed_at_f32,
            ),
        }
    }

    pub(crate) fn read_gui_config(&self) -> GUIConfig {
        GUIConfig {
            interpolation_duration: Duration::from_secs_f32(self.interpolation_duration.unmodulated_plain_value()),
            interpolation_curve: self.interpolation_curve.unmodulated_plain_value(),
        }
    }
}

impl PluginStaticParams {
    pub(crate) fn read_static_config(&self) -> StaticBPMDetectionConfig {
        self.read_config()
    }
}

trait DynamicPluginParameterConsumer {
    fn beats_lookback(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, u8>, param: &IntParam);

    fn on_off(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>, param: &PluginOnOffParam);
}

struct DynamicPluginParameterMapping<'params, Consumer> {
    params: &'params PluginDynamicParams,
    consumer: Consumer,
}

impl<Consumer: DynamicPluginParameterConsumer> DynamicPluginParameterMapping<'_, Consumer> {
    fn visit(params: &PluginDynamicParams, consumer: Consumer) {
        let mut mapping = DynamicPluginParameterMapping { params, consumer };

        DynamicBPMDetectionConfig::PARAMETERS.visit(&mut mapping);
    }
}

impl<Consumer: DynamicPluginParameterConsumer> DynamicBPMDetectionParameterVisitor<DynamicBPMDetectionConfig>
    for DynamicPluginParameterMapping<'_, Consumer>
{
    fn beats_lookback(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, u8>) {
        self.consumer.beats_lookback(parameter, &self.params.beats_lookback);
    }

    fn normal_distribution_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.normal_distribution_weight);
    }

    fn time_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.time_distance_weight);
    }

    fn velocity_current_note_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.velocity_current_note_weight);
    }

    fn velocity_note_from_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.velocity_note_from_weight);
    }

    fn in_beat_range_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.in_beat_range_weight);
    }

    fn multiplier_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.multiplier_weight);
    }

    fn subdivision_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.subdivision_weight);
    }

    fn octave_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.octave_distance_weight);
    }

    fn pitch_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.pitch_distance_weight);
    }

    fn high_tempo_bias_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.consumer.on_off(parameter, &self.params.high_tempo_bias_weight);
    }
}

struct DynamicRemoteControlParams<'page, Page> {
    page: &'page mut Page,
}

impl<Page: RemoteControlsPage> DynamicRemoteControlParams<'_, Page> {
    fn add_plugin_on_off_param(&mut self, param: &PluginOnOffParam) {
        self.page.add_param(param.param());
    }
}

impl<Page: RemoteControlsPage> DynamicPluginParameterConsumer for DynamicRemoteControlParams<'_, Page> {
    fn beats_lookback(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, u8>, param: &IntParam) {
        self.page.add_param(param);
    }

    fn on_off(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>, param: &PluginOnOffParam) {
        self.add_plugin_on_off_param(param);
    }
}

struct DynamicHostConfigReader<'config> {
    config: &'config mut DynamicBPMDetectionConfig,
}

impl DynamicPluginParameterConsumer for DynamicHostConfigReader<'_> {
    fn beats_lookback(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, u8>, param: &IntParam) {
        (parameter.set)(self.config, param.unmodulated_plain_value() as u8);
    }

    fn on_off(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>, param: &PluginOnOffParam) {
        (parameter.set)(self.config, param.read());
    }
}

struct HostParameterChangeMarker {
    current_sample: Arc<AtomicUsize>,
    changed_at: DeferredConfigUpdate,
}

impl HostParameterChangeMarker {
    fn new(current_sample: Arc<AtomicUsize>, changed_at: DeferredConfigUpdate) -> Self {
        Self { current_sample, changed_at }
    }

    fn callback<T>(&self) -> Arc<dyn Fn(T) + Send + Sync>
    where
        T: 'static + Send,
    {
        let current_sample = self.current_sample.clone();
        let changed_at = self.changed_at.clone();
        Arc::new(move |_: T| {
            changed_at.mark_changed_at_if_idle(current_sample.load(Ordering::Relaxed));
        })
    }
}

impl MidiBpmDetectorParams {
    pub fn new(
        config: &mut PluginConfig,
        static_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        gui_config_changed_at: &DeferredConfigUpdate,
        dynamic_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        current_sample: &Arc<AtomicUsize>,
        daw_port: &ArcAtomicOptionNonZeroU16,
    ) -> Self {
        let static_change_marker =
            HostParameterChangeMarker::new(current_sample.clone(), static_bpm_detection_config_changed_at.clone());
        let gui_change_marker = HostParameterChangeMarker::new(current_sample.clone(), gui_config_changed_at.clone());
        let dynamic_change_marker =
            HostParameterChangeMarker::new(current_sample.clone(), dynamic_bpm_detection_config_changed_at.clone());
        let update_static_changed_at_f32 = static_change_marker.callback();
        let update_static_changed_at_i32 = static_change_marker.callback();

        Self {
            editor_state: EguiState::from_size(1200, 600),
            send_tempo: send_tempo_param(&config.send_tempo),
            gui_params: PluginGUIParams::new(&config.gui_config, &gui_change_marker),
            static_params: PluginStaticParams::new(
                &config.static_bpm_detection_config,
                &update_static_changed_at_f32,
                &update_static_changed_at_i32,
            ),
            dynamic_params: PluginDynamicParams::new(&config.dynamic_bpm_detection_config, &dynamic_change_marker),
            daw_port: daw_port_param(daw_port),
        }
    }
}

fn send_tempo_param(send_tempo: &SendTempoOutputState) -> BoolParam {
    BoolParam::new("Send tempo", send_tempo.enabled()).with_callback(Arc::new({
        let send_tempo = send_tempo.clone();
        move |value| {
            send_tempo.set_from_host(value);
        }
    }))
}

fn daw_port_param(daw_port: &ArcAtomicOptionNonZeroU16) -> IntParam {
    IntParam::new("DAW Port", 0, IntRange::Linear { min: 0, max: 65535 }).non_automatable().with_callback(Arc::new({
        let daw_port = daw_port.clone();
        move |value| {
            daw_port.store(NonZeroU16::new(value.to_u16().unwrap()), Ordering::Relaxed);
        }
    }))
}

#[cfg(test)]
#[path = "../tests/unit/plugin_parameters.rs"]
mod tests;
