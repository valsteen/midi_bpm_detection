use std::{cell::RefCell, fmt::Debug, sync::atomic::Ordering};

use eframe::{
    egui,
    egui::{Slider, SliderClamping},
};
use errors::{LogErrorWithExt, error_backtrace};
use parameter::{Asf64, OnOff, Parameter};
use sync::ArcAtomicOptional;

/* -------------------------------------------------------------------------
 *               Basic helper functions for adding sliders
 * ------------------------------------------------------------------------- */

/// Creates an egui slider (with the given range, step, etc.) and adds it to the UI.
///
/// - `ui`: The egui UI we’re drawing into.
/// - `enabled`: Whether the slider should be greyed out or active.
/// - `parameter`: The `Parameter<S, G>` holding range, step, unit, etc.
/// - `get_set_value`: A closure that either fetches the current value (when passed `None`)
///   or sets a new value (when passed `Some(f64)`). It should return the new/updated value
///   as an `f64`.
pub fn add_slider<GuiValueType: Asf64, Config, ConfigValueType>(
    ui: &mut egui::Ui,
    enabled: bool,
    parameter: &Parameter<Config, ConfigValueType>,
    get_set_value: impl FnMut(Option<f64>) -> f64,
) {
    let mut slider = Slider::from_get_set(parameter.range.clone(), get_set_value)
        .logarithmic(parameter.logarithmic)
        .step_by(parameter.step)
        // https://github.com/emilk/egui/issues/5811
        .clamping(SliderClamping::Edits);

    if let Some(unit) = parameter.unit.as_ref() {
        slider = slider.text(*unit);
    }

    ui.add_enabled(enabled, slider);
    ui.end_row();
}

/// A convenience wrapper that performs error handling.
///
/// - The closure `get_set_as_f64` returns a `Result<ValueType, E>` so we can log or handle errors.
pub fn add_slider_default<ErrT, GuiValueType, Config, ConfigValueType>(
    ui: &mut egui::Ui,
    parameter: &Parameter<Config, ConfigValueType>,
    mut get_set_as_f64: impl FnMut(Option<GuiValueType>) -> Result<GuiValueType, ErrT>,
) where
    ErrT: Debug,
    GuiValueType: Debug + Asf64,
{
    ui.label(parameter.label);

    add_slider::<GuiValueType, Config, ConfigValueType>(ui, true, parameter, move |value_opt: Option<f64>| {
        match (value_opt, get_set_as_f64(value_opt.map(GuiValueType::new_from))) {
            // We get or set a value, and it was successful
            (_, Ok(new_value)) => new_value.as_f64(),

            // We set a value, but could not convert it
            (Some(f64_val), Err(e)) => {
                error_backtrace!("could not set parameter: {e:?}");
                f64_val
            }

            // If we didn't set a new value, we don't expect an error. Log and panic
            (None, Err(unexpected)) => {
                error_backtrace!("{unexpected:?}");
                unreachable!()
            }
        }
    });
}

/* -------------------------------------------------------------------------
 *            SlideAdderRefCell + SlideAdder + SlideAdderConfig
 *
 *   These types let you:
 *   1) store references to the UI and your application state,
 *   2) define how changes are applied,
 *   3) allow building multiple sliders for different configs
 *      in the same UI section.
 * ------------------------------------------------------------------------- */

/// Holds all data needed to add sliders to the UI:
/// - a mutable reference to the UI,
/// - a closure (`apply_fn`) that receives an `Applier`, and decides to use it depending on locally
///   available information.
/// - a mutable reference to that `Applier`.
///
/// We wrap this in a `RefCell` (`SliderAdderRefCell`) so we can have multiple
/// `SlideAdder` referencing the same mutable `ui`, but configured with different `apply_fn`.
pub struct SlideAdder<'ui, 'state, Applier, ApplyFn, ErrT>
where
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    ErrT: Debug,
{
    /// The egui UI we’re building widgets into.
    ui: &'ui mut egui::Ui,

    /// A function/closure that will be called with the `applier` after a slider sets a new value,
    /// and decide based locally available state whether to use the `applier` to propagate changes.
    apply_fn: ApplyFn,

    /// The Applier is used to propagate the changes, typically triggering a re-calculation of
    /// the represented data given new parameters.
    applier: &'state mut Applier,
}

/// Just a convenience newtype around `RefCell<SlideAdder<...>>`.
/// This is so you can hand out references to a `SlideAdder` but still
/// mutate the inner struct (its `ui`, the `applier`, etc.) in your code.
pub struct SliderAdderRefCell<'ui, 'state, Applier, ApplyFn, ErrT>(
    RefCell<SlideAdder<'ui, 'state, Applier, ApplyFn, ErrT>>,
)
where
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    ErrT: Debug;

impl<'ui, 'state, Applier, ApplyFn, ErrT> SlideAdder<'ui, 'state, Applier, ApplyFn, ErrT>
where
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    ErrT: Debug,
{
    /// Returns a RefCell‐wrapped `SlideAdder`, so we can hold several `SlideAdder` that mutably
    /// reference the same `ui`, `apply_fn` and `applier`. We can then have several `SliderAdder`
    /// with different `apply_fn` but adding sliders to the same `ui`.
    pub fn builder(
        ui: &'ui mut egui::Ui,
        apply_fn: ApplyFn,
        applier: &'state mut Applier,
    ) -> SliderAdderRefCell<'ui, 'state, Applier, ApplyFn, ErrT> {
        SliderAdderRefCell(RefCell::new(Self { ui, apply_fn, applier }))
    }
}

impl<'ui, 'state, Applier, ApplyFn, ErrT> SliderAdderRefCell<'ui, 'state, Applier, ApplyFn, ErrT>
where
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    ErrT: Debug,
{
    /// Creates a `SlideAdderConfig` that knows how to get a config `Config` out of `Applier`
    /// using `get_config`.
    pub fn for_config<'_self, GetConfig, Config>(
        &'_self self,
        get_config: GetConfig,
    ) -> SlideAdderConfig<'ui, 'state, '_self, Applier, ApplyFn, Config, GetConfig, ErrT>
    where
        GetConfig: Fn(&mut Applier) -> &mut Config + Copy,
    {
        SlideAdderConfig { slide_adder: self, config_getter: get_config }
    }
}

/* -------------------------------------------------------------------------
 *            SlideAdderConfig
 *
 *   Wraps the knowledge of how to get a config out of the main Applier.
 *   Then it provides methods `.add(...)`, `.add_atomic_u8(...)`,
 *   `.add_on_off(...)` that add specific parameter types.
 * ------------------------------------------------------------------------- */

/// This type holds:
/// - a reference to the `SlideAdderRefCell` (so we can access `ui`, `applier`, etc.),
/// - a function `get_config` to retrieve a `Config` from our top‐level `Applier`.
///
/// Then we have convenience methods to add different types of sliders.
pub struct SlideAdderConfig<'ui, 'state, '_self, Applier, ApplyFn, Config, GetConfig, ErrT>
where
    ErrT: Debug,
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    GetConfig: Fn(&mut Applier) -> &mut Config + Copy,
{
    /// The `SlideAdder` is in a `RefCell` so we can mutably borrow it from `&self`.
    slide_adder: &'_self SliderAdderRefCell<'ui, 'state, Applier, ApplyFn, ErrT>,

    /// A closure that takes the top‐level `Applier` and returns
    /// the config object we actually want to edit with sliders.
    config_getter: GetConfig,
}

impl<Applier, ApplyFn, Config, GetConfig, ErrT> SlideAdderConfig<'_, '_, '_, Applier, ApplyFn, Config, GetConfig, ErrT>
where
    ErrT: Debug,
    ApplyFn: for<'any> Fn(&'any mut Applier) -> Result<(), ErrT> + Copy,
    GetConfig: Fn(&mut Applier) -> &mut Config + Copy,
{
    /// Adds a standard slider for the given `parameter`.
    /// `parameter.get_mut` says how to get `&mut ValueType` inside the `Config`.
    pub fn add<ValueType>(&mut self, parameter: &Parameter<Config, ValueType>)
    where
        ValueType: Copy + Asf64 + Debug,
    {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();

        add_slider_default(slide_adder.ui, parameter, |value_opt: Option<ValueType>| {
            let config = (self.config_getter)(slide_adder.applier);
            match value_opt {
                // No new value => just return the current one
                None => Ok::<_, ErrT>(*(parameter.get_mut)(config)),

                // Some new value => set it, then call `apply_fn`
                Some(new_value) => {
                    *(parameter.get_mut)(config) = new_value;
                    (slide_adder.apply_fn)(slide_adder.applier)?;
                    Ok(new_value)
                }
            }
        });
    }

    /// A specialized variant that sets an `ArcAtomicOptional<u8>` inside the config.
    pub fn add_atomic_u8(&mut self, parameter: &Parameter<Config, ArcAtomicOptional<u8>>) {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();
        let atomic_u8 = &*(parameter.get_mut)((self.config_getter)(slide_adder.applier));

        add_slider_default(slide_adder.ui, parameter, |value_opt: Option<u8>| {
            Ok::<_, ErrT>(match value_opt {
                None => atomic_u8.load(Ordering::Relaxed).unwrap_or_default(),
                Some(new_u8) => {
                    atomic_u8.store(Some(new_u8), Ordering::Relaxed);
                    new_u8
                }
            })
        });
    }

    /// A specialized variant that toggles an `OnOff<V>` via a button, then
    /// optionally enables a slider when `OnOff::On(...)`.
    pub fn add_on_off<ValueType>(&mut self, parameter: &Parameter<Config, OnOff<ValueType>>)
    where
        ValueType: Asf64 + Copy,
    {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();

        #[cfg(feature = "on_off_widgets")]
        let (is_enabled, changed) = {
            let mut is_enabled = (parameter.get_mut)((self.config_getter)(slide_adder.applier)).is_enabled();
            let on_off_checkbox = slide_adder.ui.checkbox(&mut is_enabled, parameter.label);
            (is_enabled, on_off_checkbox.changed())
        };
        // on/off checkbox don't work in plugin mode, because the plugin host doesn't know about
        // values that can be disabled and come back as '0'
        #[cfg(not(feature = "on_off_widgets"))]
        let (is_enabled, changed) = {
            slide_adder.ui.label(parameter.label);
            (true, false)
        };

        // We either add an enabled or disabled slider based on the new state:
        add_slider::<ValueType, _, _>(slide_adder.ui, is_enabled, parameter, |new_val_f64| {
            let config = (self.config_getter)(slide_adder.applier);
            let current_value_mut = (parameter.get_mut)(config);

            if changed {
                assert!(new_val_f64.is_none(), "unexpected simultaneous change of value and checkbox state");
                current_value_mut.set_enabled(is_enabled);
                let return_value = current_value_mut.value().as_f64();
                (slide_adder.apply_fn)(slide_adder.applier).log_error_msg("could not apply parameter").ok();
                return return_value;
            }

            if let (true, Some(f64_val)) = (is_enabled, new_val_f64) {
                // this slider is enabled and a new value has been received: apply the change
                let new_value = ValueType::new_from(f64_val);
                *current_value_mut = OnOff::On(new_value);
                (slide_adder.apply_fn)(slide_adder.applier).log_error_msg("could not apply parameter").ok();
                return new_value.as_f64();
            }
            current_value_mut.value().as_f64()
        });
    }
}
