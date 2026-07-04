#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use std::{ops::RangeInclusive, time::Duration};

pub use parameter_macros::parameter_group;

/// Static, config-free metadata for a typed parameter.
///
/// This is the canonical metadata contract used by the `#[parameter_group]`
/// proc macro. The macro generates one `ParameterSpec<T>` per annotated config
/// field, then builds `Parameter<Config, T>` values by pairing this spec with
/// generated `get`/`set` accessors.
///
/// Keep this shape coordinated with the `parameter_group` macro backend:
/// changing these fields changes the generated parameter contract.
pub struct ParameterSpec<ValueType> {
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub range: RangeInclusive<f64>,
    pub step: f64,
    pub logarithmic: bool,
    pub default: ValueType,
}

pub struct Parameter<Config, ValueType> {
    pub spec: ParameterSpec<ValueType>,
    pub get: fn(&Config) -> ValueType,
    pub set: fn(&mut Config, ValueType),
}

/// Source field identity paired with generated typed parameter metadata.
///
/// `field_name` is the Rust config field name from the `#[parameter_group]`
/// input. It is generic traversal metadata, not a plugin host ID contract.
pub struct ParameterField<Config, ValueType> {
    pub field_name: &'static str,
    pub parameter: Parameter<Config, ValueType>,
}

pub trait ParameterFieldDescriptor<Config> {
    type Value: Asf64;

    const FIELD_NAME: &'static str;

    #[must_use]
    fn parameter() -> Parameter<Config, Self::Value>;

    #[must_use]
    fn field() -> ParameterField<Config, Self::Value> {
        ParameterField { field_name: Self::FIELD_NAME, parameter: Self::parameter() }
    }
}

impl<Config, ValueType> Parameter<Config, ValueType> {
    pub const fn new(
        spec: ParameterSpec<ValueType>,
        get: fn(&Config) -> ValueType,
        set: fn(&mut Config, ValueType),
    ) -> Self {
        Self { spec, get, set }
    }
}

impl<Config, ValueType: Asf64> Parameter<Config, ValueType> {
    pub fn validate_config_value(&self, config: &Config) -> Result<(), String> {
        let value = (self.get)(config).as_f64();
        if self.spec.range.contains(&value) {
            return Ok(());
        }

        Err(format!(
            "{} value {value} is outside declared range {}..={}",
            self.spec.label,
            self.spec.range.start(),
            self.spec.range.end()
        ))
    }
}

pub trait Asf64 {
    fn as_f64(&self) -> f64;
    fn set_from_f64(&mut self, value: f64);
    fn new_from(value: f64) -> Self;
}

impl Asf64 for u128 {
    fn as_f64(&self) -> f64 {
        *self as f64
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = value as u128;
    }

    fn new_from(value: f64) -> Self {
        value as u128
    }
}

impl Asf64 for f32 {
    fn as_f64(&self) -> f64 {
        From::from(*self)
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = value as f32;
    }

    fn new_from(value: f64) -> Self {
        value as f32
    }
}

impl Asf64 for f64 {
    fn as_f64(&self) -> f64 {
        *self
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = value;
    }

    fn new_from(value: f64) -> Self {
        value
    }
}

impl Asf64 for u8 {
    fn as_f64(&self) -> f64 {
        From::from(*self)
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = value as u8;
    }

    fn new_from(value: f64) -> Self {
        value as u8
    }
}

impl Asf64 for u16 {
    #[inline]
    fn as_f64(&self) -> f64 {
        From::from(*self)
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = value as u16;
    }

    fn new_from(value: f64) -> Self {
        value as u16
    }
}

impl Asf64 for Duration {
    fn as_f64(&self) -> f64 {
        self.as_secs_f64()
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = Duration::from_secs_f64(value);
    }

    fn new_from(value: f64) -> Self {
        Duration::from_secs_f64(value)
    }
}
