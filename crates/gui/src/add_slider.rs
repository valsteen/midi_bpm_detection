use eframe::{
    egui,
    egui::{Slider, SliderClamping},
};
use errors::{LogErrorWithExt, error_backtrace};
use parameter::{Asf64, OnOff, Parameter};
use std::{cell::RefCell, fmt::Debug, sync::atomic::Ordering};
use sync::ArcAtomicOptional;

pub fn add_slider<V: Asf64, S, G>(
    ui: &mut egui::Ui,
    enabled: bool,
    parameter: &Parameter<S, G>,
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

pub fn add_slider_default<E, V, S, G>(
    ui: &mut egui::Ui,
    parameter: &Parameter<S, G>,
    mut get_set_as_f64: impl FnMut(Option<V>) -> Result<V, E>,
) where
    E: Debug,
    V: Debug + Asf64,
{
    ui.label(parameter.label);

    add_slider::<V, S, G>(ui, true, parameter, move |value: Option<f64>| {
        match (value, get_set_as_f64(value.map(|value| V::from(value)))) {
            (_, Ok(value)) => value.get(),
            (Some(value), Err(e)) => {
                error_backtrace!("could not set parameter: {e:?}");
                value
            }

            unexpected => {
                error_backtrace!("{unexpected:?}");
                unreachable!()
            }
        }
    });
}

pub struct SlideAdder<'a, 'b, A, F, E>
where
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    E: Debug,
{
    ui: &'a mut egui::Ui,
    apply: F,
    applier: &'b mut A,
}

impl<'a, 'b, A, F, E> SlideAdder<'a, 'b, A, F, E>
where
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    E: Debug,
{
    pub fn builder(ui: &'a mut egui::Ui, apply: F, applier: &'b mut A) -> SliderAdderRefCell<'a, 'b, A, F, E>
    where
        F: for<'i> Fn(&mut A) -> Result<(), E> + Copy,
    {
        SliderAdderRefCell(RefCell::new(Self { ui, apply, applier }))
    }
}

impl<'a, 'b, A, F, E> SliderAdderRefCell<'a, 'b, A, F, E>
where
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    E: Debug,
{
    pub fn for_config<'s, GetConfig, C>(
        &'s self,
        get_config: GetConfig,
    ) -> SlideAdderConfig<'a, 'b, 's, A, F, C, GetConfig, E>
    where
        GetConfig: Fn(&mut A) -> &mut C + Copy,
    {
        SlideAdderConfig { slide_adder: self, get_config }
    }
}

pub struct SliderAdderRefCell<'a, 'b, A, F, E>(RefCell<SlideAdder<'a, 'b, A, F, E>>)
where
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    E: Debug;

pub struct SlideAdderConfig<'a, 'b, 'c, A, F, C, GetConfig, E>
where
    E: Debug,
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    GetConfig: Fn(&mut A) -> &mut C + Copy,
{
    slide_adder: &'c SliderAdderRefCell<'a, 'b, A, F, E>,
    get_config: GetConfig,
}

#[allow(clippy::elidable_lifetime_names)]
impl<'a, 'b, 'c, A, F, C, GetConfig, E> SlideAdderConfig<'a, 'b, 'c, A, F, C, GetConfig, E>
where
    E: Debug,
    F: for<'i> Fn(&'i mut A) -> Result<(), E> + Copy,
    GetConfig: Fn(&mut A) -> &mut C + Copy,
{
    pub fn add<V>(&mut self, parameter: &Parameter<C, V>)
    where
        V: Copy + Asf64 + Debug,
    {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();

        add_slider_default(slide_adder.ui, parameter, {
            |value| {
                let config = (self.get_config)(slide_adder.applier);
                match value {
                    None => Ok::<_, E>(*(parameter.get_mut)(config)),
                    Some(value) => {
                        *(parameter.get_mut)(config) = value;
                        (slide_adder.apply)(slide_adder.applier)?;
                        Ok(value)
                    }
                }
            }
        });
    }

    pub fn add_atomic_u8(&mut self, parameter: &Parameter<C, ArcAtomicOptional<u8>>) {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();
        let atomic_u8 = &*(parameter.get_mut)((self.get_config)(slide_adder.applier));

        add_slider_default(slide_adder.ui, parameter, |value: Option<u8>| {
            Ok::<_, E>(match value {
                None => atomic_u8.load(Ordering::Relaxed).unwrap_or_default(),
                Some(value) => {
                    atomic_u8.store(Some(value), Ordering::Relaxed);
                    value
                }
            })
        });
    }

    pub fn add_on_off<V>(&mut self, parameter: &Parameter<C, OnOff<V>>)
    where
        V: Asf64 + Copy,
    {
        let slide_adder = &mut *self.slide_adder.0.borrow_mut();

        let button = slide_adder.ui.button(parameter.label);

        let must_enable = match (parameter.get_mut)((self.get_config)(slide_adder.applier)) {
            state @ OnOff::Off(_) => {
                if button.clicked() {
                    *state = OnOff::On(state.value());
                    true
                } else {
                    false
                }
            }
            state @ OnOff::On(_) => {
                if button.clicked() {
                    *state = OnOff::Off(state.value());
                    false
                } else {
                    true
                }
            }
        };

        add_slider::<V, _, _>(slide_adder.ui, must_enable, parameter, |new_value| {
            let config = (self.get_config)(slide_adder.applier);
            let current_value_mut = (parameter.get_mut)(config).value_mut();
            if must_enable {
                let new_value = new_value.map(V::from);
                match new_value {
                    None => {
                        let ret = *current_value_mut;
                        if button.clicked() {
                            (slide_adder.apply)(slide_adder.applier).log_error_msg("could not apply parameter").ok();
                        }
                        ret
                    }
                    Some(value) => {
                        *current_value_mut = value;
                        (slide_adder.apply)(slide_adder.applier).log_error_msg("could not apply parameter").ok();
                        value
                    }
                }
                .get()
            } else {
                let ret = current_value_mut.get();
                if button.clicked() {
                    (slide_adder.apply)(slide_adder.applier).log_error_msg("could not apply parameter").ok();
                }
                ret
            }
        });
    }
}
