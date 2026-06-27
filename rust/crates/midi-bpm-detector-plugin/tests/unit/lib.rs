use nih_plug::prelude::{ClapPlugin, Param, RemoteControlsContext, RemoteControlsPage, RemoteControlsSection};

use super::{DeferredConfigUpdate, MidiBpmDetector, PluginTiming};

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
fn deferred_config_update_names_initial_sync_sample() {
    let update = DeferredConfigUpdate::pending_initial_sync();

    assert_eq!(update.changed_at_sample(), Some(1));
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
fn normal_distribution_remote_controls_match_canonical_settings_order() {
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
