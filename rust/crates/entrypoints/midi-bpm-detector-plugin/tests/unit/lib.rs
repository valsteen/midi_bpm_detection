use std::sync::{Mutex, atomic::Ordering};

use nice_plug::{
    context::PluginApi,
    midi::PluginNoteEvent,
    prelude::{
        AuxiliaryBuffers, Buffer, ClapPlugin, Param, Params, Plugin, ProcessContext, RemoteControlsContext,
        RemoteControlsPage, RemoteControlsSection, Transport,
    },
};
use parameter_on_off::OnOff;

use super::{
    DeferredConfigUpdate, MidiBpmDetector, PARAMETER_SYNC_COALESCING_WINDOW, PluginTiming, duration_to_sample,
    task_executor::Task,
};

#[derive(Default)]
struct RemoteControlContext {
    sections: Vec<RemoteControlSectionSnapshot>,
}

struct RemoteControlSection {
    name: String,
    pages: Vec<RemoteControlPageSnapshot>,
}

struct RemoteControlPage {
    name: String,
    params: Vec<String>,
}

struct RemoteControlSectionSnapshot {
    name: String,
    pages: Vec<RemoteControlPageSnapshot>,
}

struct RemoteControlPageSnapshot {
    name: String,
    params: Vec<String>,
}

impl RemoteControlsContext for RemoteControlContext {
    type Section = RemoteControlSection;

    fn add_section(&mut self, name: impl Into<String>, f: impl FnOnce(&mut Self::Section)) {
        let mut section = RemoteControlSection { name: name.into(), pages: Vec::new() };
        f(&mut section);
        self.sections.push(RemoteControlSectionSnapshot { name: section.name, pages: section.pages });
    }
}

impl RemoteControlsSection for RemoteControlSection {
    type Page = RemoteControlPage;

    fn add_page(&mut self, name: impl Into<String>, f: impl FnOnce(&mut Self::Page)) {
        let mut page = RemoteControlPage { name: name.into(), params: Vec::new() };
        f(&mut page);
        self.pages.push(RemoteControlPageSnapshot { name: page.name, params: page.params });
    }
}

impl RemoteControlsPage for RemoteControlPage {
    fn add_param(&mut self, param: &impl Param) {
        self.params.push(param.name().to_owned());
    }

    fn add_spacer(&mut self) {}
}

#[test]
fn delay_has_not_elapsed_before_target_sample() {
    assert!(!MidiBpmDetector::has_delay_elapsed(14, 10, 5));
}

#[test]
fn delay_has_elapsed_at_target_sample() {
    assert!(MidiBpmDetector::has_delay_elapsed(15, 10, 5));
}

#[test]
fn delay_uses_saturating_addition() {
    assert!(!MidiBpmDetector::has_delay_elapsed(usize::MAX - 1, usize::MAX - 1, 10));
    assert!(MidiBpmDetector::has_delay_elapsed(usize::MAX, usize::MAX - 1, 10));
}

#[test]
fn plugin_timing_has_no_sample_rate_before_host_initialization() {
    let timing = PluginTiming::default();

    assert_eq!(timing.sample_rate(), None);
}

#[test]
fn plugin_timing_exposes_sample_rate_after_host_initialization() {
    let mut timing = PluginTiming::default();

    assert!(timing.initialize(48_000.0));

    assert_eq!(timing.sample_rate(), Some(48_000));
}

#[test]
fn plugin_timing_rejects_zero_sample_rate() {
    let mut timing = PluginTiming::default();

    assert!(!timing.initialize(0.0));

    assert_eq!(timing.sample_rate(), None);
}

#[test]
fn deferred_config_update_preserves_first_change_sample_until_taken() {
    let update = DeferredConfigUpdate::idle();

    update.mark_changed_at_if_idle(8);
    update.mark_changed_at_if_idle(13);

    assert_eq!(update.changed_at_sample(), Some(8));
    assert_eq!(update.take(), Some(8));
    assert_eq!(update.changed_at_sample(), None);
}

#[test]
fn due_config_update_is_taken_once() {
    let update = DeferredConfigUpdate::idle();
    update.mark_changed_at_if_idle(10);

    assert!(MidiBpmDetector::take_config_update_ready_for_dispatch(15, 5, &update));
    assert!(!MidiBpmDetector::take_config_update_ready_for_dispatch(15, 5, &update));
}

#[test]
fn config_update_remains_pending_before_due_sample() {
    let update = DeferredConfigUpdate::idle();
    update.mark_changed_at_if_idle(10);

    assert!(!MidiBpmDetector::take_config_update_ready_for_dispatch(14, 5, &update));
    assert_eq!(update.changed_at_sample(), Some(10));
}

#[test]
fn normal_distribution_remote_controls_match_canonical_settings_order() {
    std::thread::Builder::new()
        .name(String::from("normal_distribution_remote_controls"))
        .stack_size(32 * 1024 * 1024)
        .spawn(assert_normal_distribution_remote_controls_match_canonical_settings_order)
        .expect("normal distribution remote-control test thread should start")
        .join()
        .expect("normal distribution remote-control test should not panic");
}

fn assert_normal_distribution_remote_controls_match_canonical_settings_order() {
    let plugin = MidiBpmDetector::default();
    let mut context = RemoteControlContext::default();

    ClapPlugin::remote_controls(&plugin, &mut context);

    let static_section = context
        .sections
        .iter()
        .find(|section| section.name == "Static parameters")
        .expect("static parameters section should exist");
    let normal_distribution_page = static_section
        .pages
        .iter()
        .find(|page| page.name == "Normal distribution")
        .expect("normal distribution page should exist");

    assert_eq!(
        normal_distribution_page.params,
        ["Standard deviation", "Normal distribution resolution", "Normal distribution cutoff", "factor",]
    );
}

#[test]
fn enabled_only_host_automation_dispatches_updated_dynamic_config() {
    std::thread::Builder::new()
        .name(String::from("enabled_only_dynamic_dispatch"))
        .stack_size(32 * 1024 * 1024)
        .spawn(assert_enabled_only_host_automation_dispatches_updated_dynamic_config)
        .expect("enabled-only dynamic-dispatch test thread should start")
        .join()
        .expect("enabled-only dynamic-dispatch test should not panic");
}

fn assert_enabled_only_host_automation_dispatches_updated_dynamic_config() {
    let mut plugin = MidiBpmDetector::default();
    let _ = plugin.static_bpm_detection_config_changed_at.take();
    let _ = plugin.gui_config_changed_at.take();
    let _ = plugin.dynamic_bpm_detection_config_changed_at.take();
    assert!(plugin.timing.initialize(48_000.0));

    let before = plugin.params.dynamic_params.normal_distribution_weight.read();
    let enabled = !before.is_enabled();
    let enabled_param = plugin
        .params
        .dynamic_params
        .param_map()
        .into_iter()
        .find_map(|(id, param, _)| (id == "normal_distribution_weight_enabled").then_some(param))
        .expect("normal distribution enabled parameter should exist");

    unsafe {
        enabled_param._internal_set_normalized_value(if enabled { 1.0 } else { 0.0 });
    }
    assert_eq!(plugin.params.dynamic_params.normal_distribution_weight.read(), OnOff::new(enabled, before.value()));

    plugin.current_sample.store(duration_to_sample(48_000, PARAMETER_SYNC_COALESCING_WINDOW), Ordering::Relaxed);
    let mut buffer = Buffer::default();
    let mut auxiliary_inputs: [Buffer<'_>; 0] = [];
    let mut auxiliary_outputs: [Buffer<'_>; 0] = [];
    let mut auxiliary_buffers = AuxiliaryBuffers { inputs: &mut auxiliary_inputs, outputs: &mut auxiliary_outputs };
    let mut context = RecordingProcessContext::new(48_000.0);

    Plugin::process(&mut plugin, &mut buffer, &mut auxiliary_buffers, &mut context);

    let tasks = context.tasks.into_inner().unwrap();
    assert_eq!(tasks.len(), 1);
    let Task::ApplyDynamicConfig(config) = &tasks[0] else {
        panic!("expected one dynamic-config task, got {:?}", tasks[0]);
    };
    assert_eq!(config.normal_distribution_weight, OnOff::new(enabled, before.value()));
}

struct RecordingProcessContext {
    transport: Transport,
    tasks: Mutex<Vec<Task>>,
}

impl RecordingProcessContext {
    fn new(sample_rate: f32) -> Self {
        Self { transport: Transport::new(sample_rate), tasks: Mutex::new(Vec::new()) }
    }
}

impl ProcessContext<MidiBpmDetector> for RecordingProcessContext {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }

    fn execute_background(&self, task: Task) {
        self.tasks.lock().unwrap().push(task);
    }

    fn execute_gui(&self, task: Task) {
        self.tasks.lock().unwrap().push(task);
    }

    fn transport(&self) -> &Transport {
        &self.transport
    }

    fn next_event(&mut self) -> Option<PluginNoteEvent<MidiBpmDetector>> {
        None
    }

    fn send_event(&mut self, _event: PluginNoteEvent<MidiBpmDetector>) {}

    fn set_latency_samples(&self, _samples: u32) {}

    fn set_current_voice_capacity(&self, _capacity: u32) {}
}
