use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, DynamicBPMDetectionParameterVisitor,
    NormalDistributionConfig, NormalDistributionConfigAccessor, NormalDistributionParameterVisitor,
    StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor, StaticBPMDetectionParameterVisitor,
};

use super::*;
use crate::config::{GUIConfig, GUIConfigAccessor, GUIParameterVisitor};

fn assert_dynamic_parameter_visitor<Config>()
where
    Config: DynamicBPMDetectionConfigAccessor,
    for<'a> SlideAdder<'a, Config>: DynamicBPMDetectionParameterVisitor<Config>,
{
}

fn assert_gui_parameter_visitor<Config>()
where
    Config: GUIConfigAccessor,
    for<'a> SlideAdder<'a, Config>: GUIParameterVisitor<Config>,
{
}

fn assert_normal_distribution_parameter_visitor<Config>()
where
    Config: NormalDistributionConfigAccessor,
    for<'a> SlideAdder<'a, Config>: NormalDistributionParameterVisitor<Config>,
{
}

fn assert_static_parameter_visitor<Config>()
where
    Config: StaticBPMDetectionConfigAccessor,
    for<'a> SlideAdder<'a, Config>: StaticBPMDetectionParameterVisitor<Config>,
{
}

#[test]
fn slide_adder_can_render_dynamic_parameter_visitor() {
    assert_dynamic_parameter_visitor::<DynamicBPMDetectionConfig>();
}

#[test]
fn slide_adder_can_render_gui_parameter_visitor() {
    assert_gui_parameter_visitor::<GUIConfig>();
}

#[test]
fn slide_adder_can_render_normal_distribution_parameter_visitor() {
    assert_normal_distribution_parameter_visitor::<NormalDistributionConfig>();
}

#[test]
fn slide_adder_can_render_static_parameter_visitor() {
    assert_static_parameter_visitor::<StaticBPMDetectionConfig>();
}
