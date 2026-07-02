use std::sync::{Arc, atomic::AtomicUsize};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterFieldVisitor, NormalDistributionConfig,
    StaticBPMDetectionConfig,
};
use nih_plug::prelude::{Param, ParamFlags, Params, RemoteControlsPage};
use parameter::{Asf64, ParameterField};

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

    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert!(params.dynamic_params.velocity_current_note_weight.is_enabled());
    assert!(!params.dynamic_params.velocity_note_from_weight.is_enabled());
}

#[test]
fn dynamic_remote_controls_expose_every_dynamic_parameter() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
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
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
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
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
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
fn dynamic_generated_field_names_match_host_parameter_ids() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);
    let param_ids = params.param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();
    let mut field_names = DynamicFieldNames(Vec::new());

    DynamicBPMDetectionConfig::PARAMETERS.visit_fields(&mut field_names);

    for field_name in field_names.0 {
        assert!(param_ids.contains(&String::from(field_name)), "{field_name} is missing from host parameter IDs");
    }
}

#[test]
fn daw_port_is_visible_non_automatable_rendezvous_parameter() {
    let mut config = PluginConfig::default();
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    let flags = params.daw_port.flags();

    assert!(flags.contains(ParamFlags::NON_AUTOMATABLE));
    assert!(!flags.contains(ParamFlags::HIDDEN));
    assert!(!flags.contains(ParamFlags::HIDE_IN_GENERIC_UI));
}

struct DynamicFieldNames(Vec<&'static str>);

impl DynamicBPMDetectionParameterFieldVisitor<DynamicBPMDetectionConfig> for DynamicFieldNames {
    fn field<ValueType: Asf64>(&mut self, field: ParameterField<DynamicBPMDetectionConfig, ValueType>) {
        self.0.push(field.field_name);
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
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert_eq!(params.dynamic_params.read_dynamic_config(), source_dynamic_config);
}

#[test]
fn static_params_read_initialized_host_values_as_static_config() {
    let source_static_config = StaticBPMDetectionConfig {
        bpm_center: 111.5,
        bpm_range: 48,
        sample_rate: 720,
        normal_distribution: NormalDistributionConfig { std_dev: 18.25, resolution: 0.5, cutoff: 128.0, factor: 32.0 },
    };
    let mut config =
        PluginConfig { static_bpm_detection_config: source_static_config.clone(), ..PluginConfig::default() };
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params =
        MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &changed_at, &current_sample, &daw_port);

    assert_eq!(params.static_params.read_static_config(), source_static_config);
}
