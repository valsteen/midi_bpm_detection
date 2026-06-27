use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterVisitor, DynamicBPMDetectionParameters,
    NormalDistributionConfig, NormalDistributionParameters, StaticBPMDetectionConfig, StaticBPMDetectionParameters,
};
use gui::{GUIConfig, GUIParameters};
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
    plugin_config::PluginConfig,
    plugin_parameter_adapters::{
        PluginOnOffParam, to_plugin_duration_param, to_plugin_float_param, to_plugin_int_param, to_plugin_on_off_param,
        to_plugin_u16_logarithmic_param,
    },
};

type DynamicConfigParameters = DynamicBPMDetectionParameters<DynamicBPMDetectionConfig>;
type GuiConfigParameters = GUIParameters<GUIConfig>;
type StaticConfigParameters = StaticBPMDetectionParameters<StaticBPMDetectionConfig>;
type NormalDistributionConfigParameters = NormalDistributionParameters<NormalDistributionConfig>;

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
    pub(crate) fn add_remote_controls(&self, page: &mut impl RemoteControlsPage) {
        let mut visitor = DynamicRemoteControlParams { params: self, page };

        DynamicConfigParameters::visit(&mut visitor);
    }

    pub(crate) fn read_dynamic_config(&self) -> DynamicBPMDetectionConfig {
        let mut config = DynamicBPMDetectionConfig::default();
        let mut visitor = DynamicHostConfigReader { params: self, config: &mut config };

        DynamicConfigParameters::visit(&mut visitor);

        config
    }
}

struct DynamicRemoteControlParams<'params, 'page, Page> {
    params: &'params PluginDynamicParams,
    page: &'page mut Page,
}

impl<Page: RemoteControlsPage> DynamicRemoteControlParams<'_, '_, Page> {
    fn add_plugin_on_off_param(&mut self, param: &PluginOnOffParam) {
        self.page.add_param(param.param());
    }
}

impl<Page: RemoteControlsPage> DynamicBPMDetectionParameterVisitor<DynamicBPMDetectionConfig>
    for DynamicRemoteControlParams<'_, '_, Page>
{
    fn beats_lookback(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, u8>) {
        self.page.add_param(&self.params.beats_lookback);
    }

    fn normal_distribution_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.normal_distribution_weight);
    }

    fn time_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.time_distance_weight);
    }

    fn velocity_current_note_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.velocity_current_note_weight);
    }

    fn velocity_note_from_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.velocity_note_from_weight);
    }

    fn in_beat_range_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.in_beat_range_weight);
    }

    fn multiplier_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.multiplier_weight);
    }

    fn subdivision_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.subdivision_weight);
    }

    fn octave_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.octave_distance_weight);
    }

    fn pitch_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.pitch_distance_weight);
    }

    fn high_tempo_bias_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.add_plugin_on_off_param(&self.params.high_tempo_bias_weight);
    }
}

struct DynamicHostConfigReader<'params, 'config> {
    params: &'params PluginDynamicParams,
    config: &'config mut DynamicBPMDetectionConfig,
}

impl DynamicHostConfigReader<'_, '_> {
    fn read_plugin_on_off_param(param: &PluginOnOffParam, config_value: &mut OnOff<f32>) {
        *config_value = param.read();
    }
}

impl DynamicBPMDetectionParameterVisitor<DynamicBPMDetectionConfig> for DynamicHostConfigReader<'_, '_> {
    fn beats_lookback(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, u8>) {
        self.config.beats_lookback = self.params.beats_lookback.unmodulated_plain_value() as u8;
    }

    fn normal_distribution_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(
            &self.params.normal_distribution_weight,
            &mut self.config.normal_distribution_weight,
        );
    }

    fn time_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.time_distance_weight, &mut self.config.time_distance_weight);
    }

    fn velocity_current_note_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(
            &self.params.velocity_current_note_weight,
            &mut self.config.velocity_current_note_weight,
        );
    }

    fn velocity_note_from_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(
            &self.params.velocity_note_from_weight,
            &mut self.config.velocity_note_from_weight,
        );
    }

    fn in_beat_range_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.in_beat_range_weight, &mut self.config.in_beat_range_weight);
    }

    fn multiplier_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.multiplier_weight, &mut self.config.multiplier_weight);
    }

    fn subdivision_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.subdivision_weight, &mut self.config.subdivision_weight);
    }

    fn octave_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.octave_distance_weight, &mut self.config.octave_distance_weight);
    }

    fn pitch_distance_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.pitch_distance_weight, &mut self.config.pitch_distance_weight);
    }

    fn high_tempo_bias_weight(&mut self, _parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        Self::read_plugin_on_off_param(&self.params.high_tempo_bias_weight, &mut self.config.high_tempo_bias_weight);
    }
}

struct UpdaterFactory {
    current_sample: Arc<AtomicUsize>,
    changed_at: DeferredConfigUpdate,
}

impl UpdaterFactory {
    fn new(current_sample: Arc<AtomicUsize>, changed_at: DeferredConfigUpdate) -> Self {
        Self { current_sample, changed_at }
    }

    fn update_changed_at<T>(&self) -> Arc<dyn Fn(T) + Send + Sync>
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

#[allow(clippy::too_many_lines)]
impl MidiBpmDetectorParams {
    pub fn new(
        config: &mut PluginConfig,
        static_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        dynamic_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        current_sample: &Arc<AtomicUsize>,
        daw_port: &ArcAtomicOptionNonZeroU16,
    ) -> Self {
        let static_updater_factory =
            UpdaterFactory::new(current_sample.clone(), static_bpm_detection_config_changed_at.clone());
        let dynamic_updater_factory =
            UpdaterFactory::new(current_sample.clone(), dynamic_bpm_detection_config_changed_at.clone());
        let update_static_changed_at_f32 = static_updater_factory.update_changed_at();
        let update_static_changed_at_u16 = static_updater_factory.update_changed_at();
        let update_dynamic_changed_at_f32 = dynamic_updater_factory.update_changed_at();
        let update_dynamic_changed_at_u8 = dynamic_updater_factory.update_changed_at();
        let dynamic_parameters = &config.dynamic_bpm_detection_config;

        Self {
            editor_state: EguiState::from_size(1200, 600),
            send_tempo: BoolParam::new("Send tempo", config.send_tempo.load(Ordering::Relaxed)).with_callback(
                Arc::new({
                    let send_tempo = config.send_tempo.clone();
                    move |value| {
                        send_tempo.store(value, Ordering::Relaxed);
                    }
                }),
            ),
            gui_params: PluginGUIParams {
                interpolation_duration: to_plugin_duration_param(
                    &GuiConfigParameters::INTERPOLATION_DURATION,
                    &config.gui_config,
                    &update_dynamic_changed_at_f32,
                ),
                interpolation_curve: to_plugin_float_param(
                    &GuiConfigParameters::INTERPOLATION_CURVE,
                    &config.gui_config,
                    &update_dynamic_changed_at_f32,
                ),
            },
            static_params: PluginStaticParams {
                bpm_center: to_plugin_float_param(
                    &StaticConfigParameters::BPM_CENTER,
                    &config.static_bpm_detection_config,
                    &update_static_changed_at_f32,
                ),
                bpm_range: to_plugin_int_param(
                    &StaticConfigParameters::BPM_RANGE,
                    &config.static_bpm_detection_config,
                    &update_static_changed_at_u16,
                ),
                sample_rate: to_plugin_u16_logarithmic_param(
                    &StaticConfigParameters::SAMPLE_RATE,
                    &config.static_bpm_detection_config,
                    &update_static_changed_at_f32,
                ),
                normal_distribution: NormalDistributionParams {
                    std_dev: to_plugin_float_param(
                        &NormalDistributionConfigParameters::STD_DEV,
                        &config.static_bpm_detection_config.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    resolution: to_plugin_float_param(
                        &NormalDistributionConfigParameters::RESOLUTION,
                        &config.static_bpm_detection_config.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    cutoff: to_plugin_float_param(
                        &NormalDistributionConfigParameters::CUTOFF,
                        &config.static_bpm_detection_config.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    factor: to_plugin_float_param(
                        &NormalDistributionConfigParameters::FACTOR,
                        &config.static_bpm_detection_config.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                },
            },
            dynamic_params: PluginDynamicParams {
                beats_lookback: to_plugin_int_param(
                    &DynamicConfigParameters::BEATS_LOOKBACK,
                    dynamic_parameters,
                    &update_dynamic_changed_at_u8,
                ),
                normal_distribution_weight: to_plugin_on_off_param(
                    "normal_distribution_weight",
                    &DynamicConfigParameters::NORMAL_DISTRIBUTION_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                time_distance_weight: to_plugin_on_off_param(
                    "time_distance_weight",
                    &DynamicConfigParameters::TIME_DISTANCE_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                velocity_current_note_weight: to_plugin_on_off_param(
                    "velocity_current_note_weight",
                    &DynamicConfigParameters::VELOCITY_CURRENT_NOTE_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                velocity_note_from_weight: to_plugin_on_off_param(
                    "velocity_note_from_weight",
                    &DynamicConfigParameters::VELOCITY_NOTE_FROM_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                in_beat_range_weight: to_plugin_on_off_param(
                    "in_beat_range_weight",
                    &DynamicConfigParameters::IN_BEAT_RANGE_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                multiplier_weight: to_plugin_on_off_param(
                    "multiplier_weight",
                    &DynamicConfigParameters::MULTIPLIER_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                subdivision_weight: to_plugin_on_off_param(
                    "subdivision_weight",
                    &DynamicConfigParameters::SUBDIVISION_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                octave_distance_weight: to_plugin_on_off_param(
                    "octave_distance_weight",
                    &DynamicConfigParameters::OCTAVE_DISTANCE_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                pitch_distance_weight: to_plugin_on_off_param(
                    "pitch_distance_weight",
                    &DynamicConfigParameters::PITCH_DISTANCE_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
                high_tempo_bias_weight: to_plugin_on_off_param(
                    "high_tempo_bias_weight",
                    &DynamicConfigParameters::HIGH_TEMPO_BIAS_WEIGHT,
                    dynamic_parameters,
                    &update_dynamic_changed_at_f32,
                ),
            },
            daw_port: IntParam::new("DAW Port", 0, IntRange::Linear { min: 0, max: 65535 }).with_callback(Arc::new({
                let daw_port = daw_port.clone();
                move |value| {
                    daw_port.store(NonZeroU16::new(value.to_u16().unwrap()), Ordering::Relaxed);
                }
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::AtomicUsize};

    use bpm_detection_core::parameters::DynamicBPMDetectionConfig;
    use nih_plug::prelude::{Param, Params, RemoteControlsPage};

    use super::*;
    use crate::DeferredConfigUpdate;

    struct RemoteControlNames(Vec<String>);

    impl RemoteControlsPage for RemoteControlNames {
        fn add_param(&mut self, param: &impl Param) {
            self.0.push(param.name().to_owned());
        }

        fn add_spacer(&mut self) {}
    }

    #[test]
    fn plugin_on_off_params_initialize_enabled_state_from_matching_config_field() {
        let mut config = PluginConfig {
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig {
                velocity_current_note_weight: OnOff::On(1.0),
                velocity_note_from_weight: OnOff::Off(2.0),
                ..DynamicBPMDetectionConfig::default()
            },
            ..PluginConfig::default()
        };
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();

        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);

        assert!(params.dynamic_params.velocity_current_note_weight.is_enabled());
        assert!(!params.dynamic_params.velocity_note_from_weight.is_enabled());
    }

    #[test]
    fn dynamic_remote_controls_expose_every_dynamic_parameter() {
        let mut config = PluginConfig::default();
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);
        let mut remote_controls = RemoteControlNames(Vec::new());

        params.dynamic_params.add_remote_controls(&mut remote_controls);

        assert_eq!(
            remote_controls.0,
            [
                "Beats Lookback",
                "Normal distribution",
                "Time distance",
                "Note velocity",
                "From note velocity",
                "In beat range",
                "Multiplier",
                "Subdivision",
                "Octave distance",
                "Pitch distance",
                "High tempo bias",
            ]
        );
    }

    #[test]
    fn static_plugin_parameter_ids_match_config_field_names() {
        let mut config = PluginConfig::default();
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);
        let param_ids = params.param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();

        assert!(param_ids.contains(&String::from("bpm_center")));
        assert!(param_ids.contains(&String::from("bpm_range")));
        assert!(!param_ids.contains(&String::from("lower_bound")));
        assert!(!param_ids.contains(&String::from("upper_bound")));
    }

    #[test]
    fn dynamic_on_off_persistent_keys_match_parameter_ids() {
        let mut config = PluginConfig::default();
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);
        let persistent_keys = params.serialize_fields().into_keys().collect::<Vec<_>>();

        for key in [
            "normal_distribution_weight_onoff",
            "time_distance_weight_onoff",
            "velocity_current_note_weight_onoff",
            "velocity_note_from_weight_onoff",
            "in_beat_range_weight_onoff",
            "multiplier_weight_onoff",
            "subdivision_weight_onoff",
            "octave_distance_weight_onoff",
            "pitch_distance_weight_onoff",
            "high_tempo_bias_weight_onoff",
        ] {
            assert!(persistent_keys.contains(&String::from(key)));
        }
    }

    #[test]
    fn dynamic_params_read_initialized_host_values_as_dynamic_config() {
        let source_dynamic_config = DynamicBPMDetectionConfig {
            beats_lookback: 13,
            normal_distribution_weight: OnOff::On(0.9),
            time_distance_weight: OnOff::On(1.3),
            velocity_current_note_weight: OnOff::On(1.1),
            velocity_note_from_weight: OnOff::Off(1.2),
            in_beat_range_weight: OnOff::Off(1.8),
            multiplier_weight: OnOff::Off(1.6),
            subdivision_weight: OnOff::On(1.7),
            octave_distance_weight: OnOff::Off(1.4),
            pitch_distance_weight: OnOff::On(1.5),
            high_tempo_bias_weight: OnOff::Off(2.1),
        };
        let mut config =
            PluginConfig { dynamic_bpm_detection_config: source_dynamic_config.clone(), ..PluginConfig::default() };
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);

        assert_eq!(params.dynamic_params.read_dynamic_config(), source_dynamic_config);
    }
}
