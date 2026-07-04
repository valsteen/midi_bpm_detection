#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

use std::{borrow::Cow, fmt, marker::PhantomData};

use parameter::Asf64;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, de::Visitor, ser::SerializeStruct};

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
                let enabled = enabled.unwrap_or(true);
                let value = value.ok_or_else(|| de::Error::missing_field("value"))?;
                Ok(if enabled { OnOff::On(value) } else { OnOff::Off(value) })
            }
        }

        deserializer.deserialize_any(OnOffVisitor { marker: PhantomData })
    }
}

impl<T> Serialize for OnOff<T>
where
    T: Serialize + Copy,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (enabled, value) = match *self {
            OnOff::On(value) => (true, value),
            OnOff::Off(value) => (false, value),
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
            Self::On(value) => *value,
            Self::Off(_) => T::one(),
        }
    }

    pub fn weight(&self) -> T {
        match self {
            Self::On(value) => *value,
            Self::Off(_) => T::zero(),
        }
    }
}

impl<T> OnOff<T>
where
    T: Copy,
{
    pub fn value(&self) -> T {
        match self {
            Self::Off(value) | Self::On(value) => *value,
        }
    }

    pub fn new(is_enabled: bool, value: T) -> Self {
        if is_enabled { Self::On(value) } else { Self::Off(value) }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled {
            *self = Self::On(self.value());
        } else {
            *self = Self::Off(self.value());
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::On(_) => true,
            Self::Off(_) => false,
        }
    }
}

impl<T> Asf64 for OnOff<T>
where
    T: Asf64 + Copy,
{
    fn as_f64(&self) -> f64 {
        self.value().as_f64()
    }

    fn set_from_f64(&mut self, value: f64) {
        *self = Self::new(self.is_enabled(), T::new_from(value));
    }

    fn new_from(value: f64) -> Self {
        Self::On(T::new_from(value))
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
