use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterVisitor, NormalDistributionConfig,
    NormalDistributionParameterVisitor, StaticBPMDetectionConfig, StaticBPMDetectionParameterVisitor,
};
use gui::GUIConfig;
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Param, Params},
    prelude::{IntRange, RemoteControlsPage},
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use sync::ArcAtomicOptionNonZeroU16;

use crate::{
    DeferredConfigUpdate,
    plugin_config::{PluginConfig, SendTempoOutputState},
    plugin_parameter_adapters::{
        PluginOnOffParam, to_plugin_duration_param, to_plugin_float_param, to_plugin_int_param, to_plugin_on_off_param,
        to_plugin_u16_logarithmic_param,
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

#[derive(Params)]
pub struct NormalDistributionParams {
    #[id = "std_dev"]
    pub std_dev: FloatParam,
    #[id = "resolution"]
    pub resolution: FloatParam,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "factor"]
    pub factor: FloatParam,
}

#[derive(Params)]
pub struct PluginStaticParams {
    #[id = "bpm_center"]
    pub bpm_center: FloatParam,
    #[id = "bpm_range"]
    pub bpm_range: IntParam,
    #[id = "sample_rate"]
    pub sample_rate: FloatParam,
    #[nested(group = "normal_distribution")]
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
    fn new(config: &StaticBPMDetectionConfig, change_marker: &HostParameterChangeMarker) -> Self {
        let update_changed_at_f32 = change_marker.callback();
        let update_changed_at_u16 = change_marker.callback();
        let static_parameters = StaticBPMDetectionConfig::PARAMETERS;

        Self {
            bpm_center: to_plugin_float_param(&static_parameters.bpm_center(), config, &update_changed_at_f32),
            bpm_range: to_plugin_int_param(&static_parameters.bpm_range(), config, &update_changed_at_u16),
            sample_rate: to_plugin_u16_logarithmic_param(
                &static_parameters.sample_rate(),
                config,
                &update_changed_at_f32,
            ),
            normal_distribution: NormalDistributionParams::new(&config.normal_distribution, &update_changed_at_f32),
        }
    }

    pub(crate) fn read_static_config(&self) -> StaticBPMDetectionConfig {
        let mut config = StaticBPMDetectionConfig::default();

        StaticPluginParameterMapping::visit(self, StaticHostConfigReader { config: &mut config });
        config.normal_distribution = self.normal_distribution.read_config();

        config
    }
}

impl NormalDistributionParams {
    fn new(config: &NormalDistributionConfig, update_changed_at_f32: &Arc<dyn Fn(f32) + Send + Sync>) -> Self {
        let normal_distribution_parameters = NormalDistributionConfig::PARAMETERS;

        Self {
            std_dev: to_plugin_float_param(&normal_distribution_parameters.std_dev(), config, update_changed_at_f32),
            resolution: to_plugin_float_param(
                &normal_distribution_parameters.resolution(),
                config,
                update_changed_at_f32,
            ),
            cutoff: to_plugin_float_param(&normal_distribution_parameters.cutoff(), config, update_changed_at_f32),
            factor: to_plugin_float_param(&normal_distribution_parameters.factor(), config, update_changed_at_f32),
        }
    }

    fn read_config(&self) -> NormalDistributionConfig {
        let mut config = NormalDistributionConfig::default();

        NormalDistributionPluginParameterMapping::visit(
            self,
            NormalDistributionHostConfigReader { config: &mut config },
        );

        config
    }
}

trait StaticPluginParameterConsumer {
    fn float(&mut self, parameter: Parameter<StaticBPMDetectionConfig, f32>, param: &FloatParam);

    fn float_u16(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>, param: &FloatParam);

    fn int(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>, param: &IntParam);
}

struct StaticPluginParameterMapping<'params, Consumer> {
    params: &'params PluginStaticParams,
    consumer: Consumer,
}

impl<Consumer: StaticPluginParameterConsumer> StaticPluginParameterMapping<'_, Consumer> {
    fn visit(params: &PluginStaticParams, consumer: Consumer) {
        let mut mapping = StaticPluginParameterMapping { params, consumer };

        StaticBPMDetectionConfig::PARAMETERS.visit(&mut mapping);
    }
}

impl<Consumer: StaticPluginParameterConsumer> StaticBPMDetectionParameterVisitor<StaticBPMDetectionConfig>
    for StaticPluginParameterMapping<'_, Consumer>
{
    fn bpm_center(&mut self, parameter: Parameter<StaticBPMDetectionConfig, f32>) {
        self.consumer.float(parameter, &self.params.bpm_center);
    }

    fn bpm_range(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
        self.consumer.int(parameter, &self.params.bpm_range);
    }

    fn sample_rate(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
        self.consumer.float_u16(parameter, &self.params.sample_rate);
    }
}

struct StaticHostConfigReader<'config> {
    config: &'config mut StaticBPMDetectionConfig,
}

impl StaticPluginParameterConsumer for StaticHostConfigReader<'_> {
    fn float(&mut self, parameter: Parameter<StaticBPMDetectionConfig, f32>, param: &FloatParam) {
        (parameter.set)(self.config, param.unmodulated_plain_value());
    }

    fn float_u16(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>, param: &FloatParam) {
        (parameter.set)(self.config, param.unmodulated_plain_value() as u16);
    }

    fn int(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>, param: &IntParam) {
        (parameter.set)(self.config, param.unmodulated_plain_value() as u16);
    }
}

trait NormalDistributionPluginParameterConsumer {
    fn float64(&mut self, parameter: Parameter<NormalDistributionConfig, f64>, param: &FloatParam);

    fn float(&mut self, parameter: Parameter<NormalDistributionConfig, f32>, param: &FloatParam);
}

struct NormalDistributionPluginParameterMapping<'params, Consumer> {
    params: &'params NormalDistributionParams,
    consumer: Consumer,
}

impl<Consumer: NormalDistributionPluginParameterConsumer> NormalDistributionPluginParameterMapping<'_, Consumer> {
    fn visit(params: &NormalDistributionParams, consumer: Consumer) {
        let mut mapping = NormalDistributionPluginParameterMapping { params, consumer };

        NormalDistributionConfig::PARAMETERS.visit(&mut mapping);
    }
}

impl<Consumer: NormalDistributionPluginParameterConsumer> NormalDistributionParameterVisitor<NormalDistributionConfig>
    for NormalDistributionPluginParameterMapping<'_, Consumer>
{
    fn std_dev(&mut self, parameter: Parameter<NormalDistributionConfig, f64>) {
        self.consumer.float64(parameter, &self.params.std_dev);
    }

    fn resolution(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.consumer.float(parameter, &self.params.resolution);
    }

    fn cutoff(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.consumer.float(parameter, &self.params.cutoff);
    }

    fn factor(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.consumer.float(parameter, &self.params.factor);
    }
}

struct NormalDistributionHostConfigReader<'config> {
    config: &'config mut NormalDistributionConfig,
}

impl NormalDistributionPluginParameterConsumer for NormalDistributionHostConfigReader<'_> {
    fn float64(&mut self, parameter: Parameter<NormalDistributionConfig, f64>, param: &FloatParam) {
        (parameter.set)(self.config, f64::from(param.unmodulated_plain_value()));
    }

    fn float(&mut self, parameter: Parameter<NormalDistributionConfig, f32>, param: &FloatParam) {
        (parameter.set)(self.config, param.unmodulated_plain_value());
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

        Self {
            editor_state: EguiState::from_size(1200, 600),
            send_tempo: send_tempo_param(&config.send_tempo),
            gui_params: PluginGUIParams::new(&config.gui_config, &gui_change_marker),
            static_params: PluginStaticParams::new(&config.static_bpm_detection_config, &static_change_marker),
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
