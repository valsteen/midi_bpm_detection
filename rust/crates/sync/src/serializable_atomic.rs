use std::{
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU16,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicUsize, Ordering},
    },
};

use atomic_traits::{Atomic, fetch};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Converts between the public `Option` value and its raw atomic representation.
///
/// The codec owns the sentinel invariant. The atomic wrapper only knows how to atomically store raw values.
pub trait AtomicOptionCodec {
    type Raw: Copy + Debug + Eq;
    type Value: Copy + Debug + Eq;

    fn encode(value: Option<Self::Value>) -> Self::Raw;
    fn decode(raw: Self::Raw) -> Option<Self::Value>;
}

/// Shared atomic `Option` wrapper backed by a concrete atomic type and a domain codec.
pub struct ArcAtomicOption<A, C> {
    inner: Arc<A>,
    codec: PhantomData<C>,
}

impl<A, C> Clone for ArcAtomicOption<A, C> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), codec: PhantomData }
    }
}

impl<A, C> Default for ArcAtomicOption<A, C>
where
    A: Atomic<Type = C::Raw>,
    C: AtomicOptionCodec,
{
    fn default() -> Self {
        Self::none()
    }
}

impl<A, C> Debug for ArcAtomicOption<A, C>
where
    A: Atomic<Type = C::Raw>,
    C: AtomicOptionCodec,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.load(Ordering::Relaxed).fmt(f)
    }
}

impl<A, C> PartialEq for ArcAtomicOption<A, C>
where
    A: Atomic<Type = C::Raw>,
    C: AtomicOptionCodec,
{
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl<A, C> Eq for ArcAtomicOption<A, C>
where
    A: Atomic<Type = C::Raw>,
    C: AtomicOptionCodec,
{
}

impl<A, C> ArcAtomicOption<A, C>
where
    A: Atomic<Type = C::Raw>,
    C: AtomicOptionCodec,
{
    #[must_use]
    pub fn new(value: Option<C::Value>) -> Self {
        Self { inner: Arc::new(A::new(C::encode(value))), codec: PhantomData }
    }

    #[must_use]
    pub fn none() -> Self {
        Self::new(None)
    }

    #[must_use]
    pub fn load(&self, order: Ordering) -> Option<C::Value> {
        C::decode(Atomic::load(&*self.inner, order))
    }

    pub fn store(&self, value: Option<C::Value>, order: Ordering) {
        Atomic::store(&*self.inner, C::encode(value), order);
    }

    pub fn store_if_none(&self, value: Option<C::Value>, order: Ordering)
    where
        A: fetch::Update<Type = C::Raw>,
    {
        let encoded = C::encode(value);
        fetch::Update::fetch_update(&*self.inner, order, order, |current| {
            C::decode(current).is_none().then_some(encoded)
        })
        .ok();
    }

    #[must_use]
    pub fn take(&self, order: Ordering) -> Option<C::Value>
    where
        A: fetch::Update<Type = C::Raw>,
    {
        let none = C::encode(None);
        fetch::Update::fetch_update(&*self.inner, order, order, |current| C::decode(current).is_some().then_some(none))
            .ok()
            .and_then(C::decode)
    }

    #[must_use]
    pub fn get_or_insert(&self, value: C::Value, order: Ordering) -> C::Value
    where
        A: fetch::Update<Type = C::Raw>,
    {
        let encoded = C::encode(Some(value));
        match fetch::Update::fetch_update(&*self.inner, order, order, |current| {
            C::decode(current).is_none().then_some(encoded)
        }) {
            Ok(previous) => C::decode(previous).unwrap_or(value),
            Err(current) => C::decode(current).expect("atomic option fetch_update returned reserved sentinel"),
        }
    }
}

#[doc(hidden)]
pub struct MaxU64OptionCodec;

impl AtomicOptionCodec for MaxU64OptionCodec {
    type Raw = u64;
    type Value = u64;

    fn encode(value: Option<Self::Value>) -> Self::Raw {
        if let Some(value) = value {
            assert_ne!(value, u64::MAX, "atomic option sentinel value is reserved");
            value
        } else {
            u64::MAX
        }
    }

    fn decode(raw: Self::Raw) -> Option<Self::Value> {
        (raw != u64::MAX).then_some(raw)
    }
}

#[doc(hidden)]
pub struct MaxUsizeOptionCodec;

impl AtomicOptionCodec for MaxUsizeOptionCodec {
    type Raw = usize;
    type Value = usize;

    fn encode(value: Option<Self::Value>) -> Self::Raw {
        if let Some(value) = value {
            assert_ne!(value, usize::MAX, "atomic option sentinel value is reserved");
            value
        } else {
            usize::MAX
        }
    }

    fn decode(raw: Self::Raw) -> Option<Self::Value> {
        (raw != usize::MAX).then_some(raw)
    }
}

#[doc(hidden)]
pub struct NonZeroU16OptionCodec;

impl AtomicOptionCodec for NonZeroU16OptionCodec {
    type Raw = u16;
    type Value = NonZeroU16;

    fn encode(value: Option<Self::Value>) -> Self::Raw {
        value.map_or(0, NonZeroU16::get)
    }

    fn decode(raw: Self::Raw) -> Option<Self::Value> {
        NonZeroU16::new(raw)
    }
}

/// Atomic `Option<u64>` wrapper. `Some(0)` is a valid value.
pub type ArcAtomicOptionU64 = ArcAtomicOption<AtomicU64, MaxU64OptionCodec>;

/// Atomic `Option<usize>` wrapper for sample indexes. `Some(0)` is a valid value.
pub type ArcAtomicOptionUsize = ArcAtomicOption<AtomicUsize, MaxUsizeOptionCodec>;

/// Atomic `Option<NonZeroU16>` wrapper for port-like domains where `0` means disabled.
pub type ArcAtomicOptionNonZeroU16 = ArcAtomicOption<AtomicU16, NonZeroU16OptionCodec>;

#[derive(Clone, Default, Debug)]
pub struct ArcAtomicBool(Arc<AtomicBool>);

impl PartialEq for ArcAtomicBool {
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl Eq for ArcAtomicBool {}

impl Serialize for ArcAtomicBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.0.load(Ordering::SeqCst))
    }
}

impl<'de> Deserialize<'de> for ArcAtomicBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer).map(|b| ArcAtomicBool(Arc::new(AtomicBool::new(b))))
    }
}

impl ArcAtomicBool {
    #[must_use]
    pub fn new(val: bool) -> Self {
        ArcAtomicBool(Arc::new(AtomicBool::new(val)))
    }

    #[must_use]
    pub fn load(&self, order: Ordering) -> bool {
        self.0.load(order)
    }

    pub fn store(&self, val: bool, order: Ordering) {
        self.0.store(val, order);
    }

    #[must_use]
    pub fn take(&self, order: Ordering) -> bool {
        self.0.swap(false, order)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn fetch_xor(&self, val: bool, order: Ordering) -> bool {
        self.0.fetch_xor(val, order)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn compare_exchange(
        &self,
        current: bool,
        new: bool,
        success: Ordering,
        failure: Ordering,
    ) -> Result<bool, bool> {
        self.0.compare_exchange(current, new, success, failure)
    }

    #[must_use]
    pub fn weak(&self) -> Weak<AtomicBool> {
        Arc::downgrade(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_option_u64_starts_empty() {
        assert_eq!(ArcAtomicOptionU64::none().load(Ordering::Relaxed), None);
    }

    #[test]
    fn atomic_option_u64_stores_some_values_including_zero() {
        let value = ArcAtomicOptionU64::none();

        value.store(Some(0), Ordering::Relaxed);

        assert_eq!(value.load(Ordering::Relaxed), Some(0));
    }

    #[test]
    fn atomic_option_u64_get_or_insert_keeps_existing_value() {
        let value = ArcAtomicOptionU64::none();

        assert_eq!(value.get_or_insert(10, Ordering::Relaxed), 10);
        assert_eq!(value.get_or_insert(20, Ordering::Relaxed), 10);
    }

    #[test]
    fn atomic_option_u64_can_be_reset_to_none() {
        let value = ArcAtomicOptionU64::none();

        value.store(Some(10), Ordering::Relaxed);
        value.store(None, Ordering::Relaxed);

        assert_eq!(value.load(Ordering::Relaxed), None);
    }

    #[test]
    fn atomic_option_usize_stores_sample_zero() {
        let value = ArcAtomicOptionUsize::none();

        value.store(Some(0), Ordering::Relaxed);

        assert_eq!(value.load(Ordering::Relaxed), Some(0));
    }

    #[test]
    fn atomic_option_usize_store_if_none_keeps_first_value() {
        let value = ArcAtomicOptionUsize::none();

        value.store_if_none(Some(0), Ordering::Relaxed);
        value.store_if_none(Some(10), Ordering::Relaxed);

        assert_eq!(value.load(Ordering::Relaxed), Some(0));
    }

    #[test]
    fn atomic_option_non_zero_u16_treats_zero_as_unrepresentable() {
        let value = ArcAtomicOptionNonZeroU16::none();

        value.store(NonZeroU16::new(8000), Ordering::Relaxed);

        assert_eq!(value.load(Ordering::Relaxed).map(NonZeroU16::get), Some(8000));
        assert_eq!(value.take(Ordering::Relaxed).map(NonZeroU16::get), Some(8000));
        assert_eq!(value.load(Ordering::Relaxed), None);
    }

    #[test]
    #[should_panic(expected = "atomic option sentinel value is reserved")]
    fn atomic_option_u64_rejects_internal_sentinel() {
        ArcAtomicOptionU64::none().store(Some(u64::MAX), Ordering::Relaxed);
    }
}
