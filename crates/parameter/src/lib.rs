#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use std::{borrow::Cow, fmt, marker::PhantomData, ops::RangeInclusive, time::Duration};

pub use getset::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, de::Visitor, ser::SerializeStruct};

pub struct Parameter<Config, ValueType> {
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub range: RangeInclusive<f64>,
    pub step: f64,
    pub logarithmic: bool,
    pub get_mut: fn(&mut Config) -> &mut ValueType,
    pub default: ValueType,
}

impl<Config, ValueType> Parameter<Config, ValueType> {
    pub const fn new(
        label: &'static str,
        unit: Option<&'static str>,
        range: RangeInclusive<f64>,
        step: f64,
        logarithmic: bool,
        default: ValueType,
        get_mut: fn(&mut Config) -> &mut ValueType,
    ) -> Self {
        Self { label, unit, range, step, logarithmic, get_mut, default }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnOff<T> {
    On(T),
    Off(T),
}

impl<'de, T> Deserialize<'de> for OnOff<T>
where
    T: Deserialize<'de> + Asf64,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OnOffVisitor<T> {
            marker: PhantomData<T>,
        }

        impl<'de, T> Visitor<'de> for OnOffVisitor<T>
        where
            T: Deserialize<'de> + Asf64,
        {
            type Value = OnOff<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct OnOff or a floating point number for shorthand syntax")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(OnOff::On(Asf64::new_from(value)))
            }

            fn visit_map<V>(self, mut map: V) -> Result<OnOff<T>, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut enabled: Option<bool> = None;
                let mut value: Option<T> = None;

                while let Some(key) = map.next_key::<Cow<str>>()? {
                    match key.as_ref() {
                        "enabled" => {
                            if enabled.is_some() {
                                return Err(de::Error::duplicate_field("enabled"));
                            }
                            enabled = Some(map.next_value()?);
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            value = Some(map.next_value()?);
                        }
                        _ => return Err(de::Error::unknown_field(&key, &["enabled", "value"])),
                    }
                }
                let enabled = enabled.unwrap_or(true); // Default to true if not present
                let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                Ok(if enabled { OnOff::On(value) } else { OnOff::Off(value) })
            }
        }

        deserializer.deserialize_any(OnOffVisitor { marker: PhantomData })
    }
}

impl<T> Serialize for OnOff<T>
where
    T: Serialize + Copy, // Ensure T can be serialized and is Copy
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (enabled, value) = match *self {
            OnOff::On(v) => (true, v),
            OnOff::Off(v) => (false, v),
        };
        let mut state = serializer.serialize_struct("StdDevConfig", 2)?;
        state.serialize_field("enabled", &enabled)?;
        state.serialize_field("value", &value)?;
        state.end()
    }
}

impl<T> OnOff<T>
where
    T: Copy + num_traits::One + num_traits::Zero + std::ops::Mul<Output = T>,
{
    pub fn multiplier(&self) -> T {
        match self {
            OnOff::On(value) => *value,
            OnOff::Off(_) => T::one(),
        }
    }

    pub fn weight(&self) -> T {
        match self {
            OnOff::On(value) => *value,
            OnOff::Off(_) => T::zero(),
        }
    }
}

impl<T> OnOff<T>
where
    T: Copy,
{
    pub fn value(&self) -> T {
        match self {
            OnOff::Off(v) | OnOff::On(v) => *v,
        }
    }

    pub fn value_mut(&mut self) -> &mut T {
        match self {
            OnOff::Off(v) | OnOff::On(v) => v,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            *self = OnOff::On(self.value());
        } else {
            *self = OnOff::Off(self.value());
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            OnOff::On(_) => true,
            OnOff::Off(_) => false,
        }
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
