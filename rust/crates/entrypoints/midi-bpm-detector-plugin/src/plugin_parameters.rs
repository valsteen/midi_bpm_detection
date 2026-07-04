use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use gui::GUIConfig;
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Params},
    prelude::IntRange,
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter_nih_plug::nih_plugin_parameter_group;
use parameter_on_off_nih_plug::{OnOffF32Adapter, OnOffParam};
use sync::ArcAtomicOptionNonZeroU16;

use crate::{
    DeferredConfigUpdate,
    plugin_config::{PluginConfig, SendTempoOutputState},
};

#[nih_plugin_parameter_group(config = gui::GUIConfig, group = "GUI", accessor_macro = plugin_gui_params_accessors)]
pub struct PluginGUIParams {
    pub interpolation_duration: FloatParam,
    pub interpolation_curve: FloatParam,
}

#[nih_plugin_parameter_group(
    config = bpm_detection_core::parameters::DynamicBPMDetectionConfig,
    group = "DynamicParams",
    accessor_macro = plugin_dynamic_params_accessors
)]
pub struct PluginDynamicParams {
    pub beats_lookback: IntParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub normal_distribution_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub time_distance_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub velocity_current_note_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub velocity_note_from_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub in_beat_range_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub multiplier_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub subdivision_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub octave_distance_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub pitch_distance_weight: OnOffParam,
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
    pub high_tempo_bias_weight: OnOffParam,
}

#[nih_plugin_parameter_group(
    config = bpm_detection_core::parameters::NormalDistributionConfig,
    group = "normal_distribution",
    accessor_macro = normal_distribution_params_accessors
)]
pub struct NormalDistributionParams {
    pub std_dev: FloatParam,
    pub resolution: FloatParam,
    pub cutoff: FloatParam,
    pub factor: FloatParam,
}

#[nih_plugin_parameter_group(
    config = bpm_detection_core::parameters::StaticBPMDetectionConfig,
    group = "StaticParams",
    accessor_macro = plugin_static_params_accessors
)]
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
        let update_gui_changed_at_f32 = gui_change_marker.callback();
        let update_static_changed_at_f32 = static_change_marker.callback();
        let update_static_changed_at_i32 = static_change_marker.callback();
        let update_dynamic_changed_at_f32 = dynamic_change_marker.callback();
        let update_dynamic_changed_at_i32 = dynamic_change_marker.callback();

        Self {
            editor_state: EguiState::from_size(1200, 600),
            send_tempo: send_tempo_param(&config.send_tempo),
            gui_params: PluginGUIParams::new(&config.gui_config, &update_gui_changed_at_f32),
            static_params: PluginStaticParams::new(
                &config.static_bpm_detection_config,
                &update_static_changed_at_f32,
                &update_static_changed_at_i32,
            ),
            dynamic_params: PluginDynamicParams::new(
                &config.dynamic_bpm_detection_config,
                &update_dynamic_changed_at_f32,
                &update_dynamic_changed_at_i32,
            ),
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
