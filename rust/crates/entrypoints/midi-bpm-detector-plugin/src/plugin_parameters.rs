use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bpm_detection_config::{DynamicBPMDetectionConfig, GUIConfig, Settings, StaticBPMDetectionConfig};
use nice_plug::{
    editor::dpi::LogicalSize,
    params::{BoolParam, FloatParam, IntParam, Params},
    prelude::IntRange,
};
use nice_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter_nice_plug::nice_plugin_parameter_group;
use parameter_on_off_nice_plug::{OnOffF32Adapter, OnOffParams};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16};

use crate::{DeferredConfigUpdate, plugin_config::PluginConfig};

#[nice_plugin_parameter_group(
    config = bpm_detection_config::GUIConfig,
    group = "GUI"
)]
pub struct PluginGUIParams {
    pub interpolation_duration: FloatParam,
    pub interpolation_curve: FloatParam,
}

#[nice_plugin_parameter_group(
    config = bpm_detection_config::DynamicBPMDetectionConfig,
    group = "DynamicParams"
)]
pub struct PluginDynamicParams {
    #[nice_plugin_parameter(remote_control = "spacer_after")]
    pub beats_lookback: IntParam,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub normal_distribution_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub time_distance_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub velocity_current_note_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub velocity_note_from_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub in_beat_range_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub multiplier_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub subdivision_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub octave_distance_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub pitch_distance_weight: OnOffParams,
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub high_tempo_bias_weight: OnOffParams,
}

#[nice_plugin_parameter_group(
    config = bpm_detection_config::NormalDistributionConfig,
    group = "normal_distribution"
)]
pub struct NormalDistributionParams {
    pub std_dev: FloatParam,
    pub resolution: FloatParam,
    pub cutoff: FloatParam,
    pub factor: FloatParam,
}

#[nice_plugin_parameter_group(
    config = bpm_detection_config::StaticBPMDetectionConfig,
    group = "StaticParams"
)]
pub struct PluginStaticParams {
    pub bpm_center: FloatParam,
    pub bpm_range: IntParam,
    #[nice_plugin_parameter(adapter = "float_u16_logarithmic")]
    pub sample_rate: FloatParam,
    #[nice_plugin_nested(group = "normal_distribution")]
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
    pub(crate) fn read_dynamic_config(&self) -> DynamicBPMDetectionConfig {
        self.read_config()
    }
}

impl PluginGUIParams {
    pub(crate) fn read_gui_config(&self) -> GUIConfig {
        self.read_config()
    }
}

impl PluginStaticParams {
    pub(crate) fn read_static_config(&self) -> StaticBPMDetectionConfig {
        self.read_config()
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

    fn mark_changed_callback(&self) -> impl Fn() + Clone + Send + Sync + 'static {
        let current_sample = self.current_sample.clone();
        let changed_at = self.changed_at.clone();
        move || {
            changed_at.mark_changed_at_if_idle(current_sample.load(Ordering::Relaxed));
        }
    }
}

impl MidiBpmDetectorParams {
    pub(crate) fn read_settings(&self) -> Settings {
        Settings {
            gui_config: self.gui_params.read_gui_config(),
            static_bpm_detection_config: self.static_params.read_static_config(),
            dynamic_bpm_detection_config: self.dynamic_params.read_dynamic_config(),
        }
    }

    pub(crate) fn read_editable_settings(&self) -> gui::EditableSettings {
        gui::EditableSettings { bpm: self.read_settings(), send_tempo: Some(self.send_tempo.value()) }
    }

    pub fn new(
        config: &PluginConfig,
        static_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        gui_config_changed_at: &DeferredConfigUpdate,
        dynamic_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        current_sample: &Arc<AtomicUsize>,
        daw_port: &ArcAtomicOptionNonZeroU16,
        send_tempo_output: ArcAtomicBool,
    ) -> Self {
        let static_config_change_marker =
            HostParameterChangeMarker::new(current_sample.clone(), static_bpm_detection_config_changed_at.clone());
        let gui_config_change_marker =
            HostParameterChangeMarker::new(current_sample.clone(), gui_config_changed_at.clone());
        let dynamic_config_change_marker =
            HostParameterChangeMarker::new(current_sample.clone(), dynamic_bpm_detection_config_changed_at.clone());
        let mark_gui_config_changed = gui_config_change_marker.mark_changed_callback();
        let mark_static_config_changed = static_config_change_marker.mark_changed_callback();
        let mark_dynamic_config_changed = dynamic_config_change_marker.mark_changed_callback();

        Self {
            editor_state: EguiState::from_size(LogicalSize::new(1200.0, 600.0)),
            send_tempo: send_tempo_param(config.send_tempo, send_tempo_output),
            gui_params: PluginGUIParams::new(&config.bpm_detection.gui_config, &mark_gui_config_changed),
            static_params: PluginStaticParams::new(
                &config.bpm_detection.static_bpm_detection_config,
                &mark_static_config_changed,
            ),
            dynamic_params: PluginDynamicParams::new(
                &config.bpm_detection.dynamic_bpm_detection_config,
                &mark_dynamic_config_changed,
            ),
            daw_port: daw_port_param(daw_port),
        }
    }
}

fn send_tempo_param(initial: bool, output: ArcAtomicBool) -> BoolParam {
    BoolParam::new("Send tempo", initial).with_callback(Arc::new(move |enabled| {
        output.store(enabled, Ordering::Relaxed);
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
