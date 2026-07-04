use std::{
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use errors::info;
use gui::BPMDetectionConfig;
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use sync::{ArcAtomicBool, RwLock};

use crate::{
    MidiBpmDetector, MidiBpmDetectorParams, Task,
    parameter_sync::{GUI_PARAMETER_SYNC_COALESCING_WINDOW, ParameterSyncOrigin},
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
    gui_task_config: Arc<RwLock<PluginConfig>>,
    async_executor: AsyncExecutor<MidiBpmDetector>,
    force_evaluate_bpm_detection: ArcAtomicBool,
    delayed_update_dynamic_bpm_detection_config: Option<Instant>,
    delayed_update_gui_config: Option<Instant>,
    delayed_update_static_bpm_detection_config: Option<Instant>,
    pub has_config_changes_via_ui: bool,
}

impl BaseConfig {
    pub fn new(
        config: PluginConfig,
        gui_task_config: Arc<RwLock<PluginConfig>>,
        async_executor: AsyncExecutor<MidiBpmDetector>,
        force_evaluate_bpm_detection: ArcAtomicBool,
        params: Arc<MidiBpmDetectorParams>,
    ) -> Self {
        Self {
            config,
            gui_task_config,
            async_executor,
            force_evaluate_bpm_detection,
            delayed_update_dynamic_bpm_detection_config: None,
            delayed_update_gui_config: None,
            delayed_update_static_bpm_detection_config: None,
            has_config_changes_via_ui: false,
            params,
        }
    }

    fn delay_static_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_static_bpm_detection_config.is_none() {
            self.delayed_update_static_bpm_detection_config = Some(Instant::now());
        }
    }

    fn delay_dynamic_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_dynamic_bpm_detection_config.is_none() {
            self.delayed_update_dynamic_bpm_detection_config = Some(Instant::now());
        }
    }

    fn delay_gui_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_gui_config.is_none() {
            self.delayed_update_gui_config = Some(Instant::now());
        }
    }

    pub fn apply_delayed_updates(&mut self) {
        if self
            .delayed_update_static_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }

            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Gui));
            self.delayed_update_static_bpm_detection_config = None;
            info!("apply static params");
        }
        if self
            .delayed_update_gui_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }
            self.async_executor.execute_background(Task::GUIConfig(ParameterSyncOrigin::Gui));
            self.delayed_update_gui_config = None;
            info!("apply GUI params");
        }
        if self
            .delayed_update_dynamic_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }
            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Gui));
            self.delayed_update_dynamic_bpm_detection_config = None;
            info!("apply dynamic params");
        }
    }
}

pub(crate) struct LiveConfig<'_self> {
    pub(crate) base_config: &'_self mut BaseConfig,
    pub(crate) param_setter: &'_self ParamSetter<'_self>,
}

normal_distribution_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.static_bpm_detection_config.normal_distribution,
    params = self.base_config.params.static_params.normal_distribution,
    param_setter = self.param_setter,
    after_set = self.base_config.delay_static_changes(),
}

plugin_dynamic_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.dynamic_bpm_detection_config,
    params = self.base_config.params.dynamic_params,
    param_setter = self.param_setter,
    after_set = self.base_config.delay_dynamic_changes(),
}

plugin_static_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.static_bpm_detection_config,
    params = self.base_config.params.static_params,
    param_setter = self.param_setter,
    after_set = self.base_config.delay_static_changes(),
}

plugin_gui_params_accessors! {
    target = LiveConfig<'_>,
    config = self.base_config.config.gui_config,
    params = self.base_config.params.gui_params,
    param_setter = self.param_setter,
    after_set = self.base_config.delay_gui_changes(),
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
