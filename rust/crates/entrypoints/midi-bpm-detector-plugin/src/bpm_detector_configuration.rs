use std::sync::Arc;

use gui::BPMDetectionConfig;
use nice_plug::prelude::ParamSetter;

use crate::{
    MidiBpmDetectorParams,
    plugin_config::PluginConfig,
    plugin_parameters::{
        NormalDistributionParams, PluginDynamicParams, PluginGUIParams, PluginStaticParams,
        normal_distribution_params_accessors, plugin_dynamic_params_accessors, plugin_gui_params_accessors,
        plugin_static_params_accessors,
    },
};

pub struct BaseConfig {
    pub config: PluginConfig,
    params: Arc<MidiBpmDetectorParams>,
}

impl BaseConfig {
    pub fn new(config: PluginConfig, params: Arc<MidiBpmDetectorParams>) -> Self {
        Self { config, params }
    }

    pub(crate) fn refresh_from_host(&mut self) {
        self.config.bpm_detection = self.params.read_settings();
    }
}

pub(crate) struct LiveConfig<'_self> {
    pub(crate) base_config: &'_self mut BaseConfig,
    pub(crate) param_setter: &'_self ParamSetter<'_self>,
}

impl LiveConfig<'_> {
    fn parameter_set(&mut self) {
        // The generated transitional accessors require a hook, but parameter callbacks now own detector scheduling.
        let _ = &mut *self.base_config;
    }
}

normal_distribution_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.bpm_detection.static_bpm_detection_config.normal_distribution,
    params = self.base_config.params.static_params.normal_distribution,
    param_setter = self.param_setter,
    after_set = self.parameter_set(),
}

plugin_dynamic_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.bpm_detection.dynamic_bpm_detection_config,
    params = self.base_config.params.dynamic_params,
    param_setter = self.param_setter,
    after_set = self.parameter_set(),
}

plugin_static_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.bpm_detection.static_bpm_detection_config,
    params = self.base_config.params.static_params,
    param_setter = self.param_setter,
    after_set = self.parameter_set(),
}

plugin_gui_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.bpm_detection.gui_config,
    params = self.base_config.params.gui_params,
    param_setter = self.param_setter,
    after_set = self.parameter_set(),
}

impl BPMDetectionConfig for LiveConfig<'_> {
    fn get_send_tempo(&self) -> bool {
        self.base_config.config.send_tempo.enabled()
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.base_config.config.send_tempo.set_from_gui(enabled);
    }
}

#[cfg(test)]
#[path = "../tests/unit/bpm_detector_configuration.rs"]
mod tests;
