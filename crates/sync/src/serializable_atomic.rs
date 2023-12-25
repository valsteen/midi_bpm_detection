use atomic::Atomic;
use bytemuck::NoUninit;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};

#[derive(Clone, Default)]
pub struct ArcAtomicOptional<T>(Arc<Atomic<T>>)
where
    T: num_traits::Zero + Debug;

impl<T> Debug for ArcAtomicOptional<T>
where
    T: num_traits::Zero + Debug + NoUninit,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = self.0.load(Ordering::Relaxed);
        if value.is_zero() {
            None::<T>.fmt(f)
        } else {
            Some(value).fmt(f)
        }
    }
}

impl<T> PartialEq for ArcAtomicOptional<T>
where
    T: num_traits::Zero + PartialEq + Debug + NoUninit,
{
    fn eq(&self, other: &Self) -> bool {
        self.load(Ordering::SeqCst) == other.load(Ordering::SeqCst)
    }
}

impl<T> Eq for ArcAtomicOptional<T> where T: num_traits::Zero + Eq + NoUninit + Debug {}

impl<T> ArcAtomicOptional<T>
where
    T: num_traits::Zero + Copy + NoUninit + Debug,
{
    #[must_use]
    pub fn new(val: Option<T>) -> Self {
        Self(Arc::new(Atomic::new(val.unwrap_or(T::zero()))))
    }

    #[must_use]
    pub fn none() -> Self {
        Self(Arc::new(Atomic::new(T::zero())))
    }

    #[must_use]
    pub fn load(&self, order: Ordering) -> Option<T> {
        let value = self.0.load(order);
        (!value.is_zero()).then_some(value)
    }

    pub fn store(&self, val: Option<T>, order: Ordering) {
        self.0.store(val.unwrap_or(T::zero()), order);
    }

    pub fn store_if_none(&self, val: Option<T>, order: Ordering) {
        self.0.fetch_update(order, order, |current| (current.is_zero()).then_some(val.unwrap_or(T::zero()))).ok();
    }

    #[must_use]
    pub fn take(&self, order: Ordering) -> Option<T> {
        self.0.fetch_update(order, order, |value| (!value.is_zero()).then_some(T::zero())).ok()
    }
}

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

    #[allow(forbidden_lint_groups)]
    #[allow(clippy::must_use_candidate)]
    pub fn fetch_xor(&self, val: bool, order: Ordering) -> bool {
        self.0.fetch_xor(val, order)
    }

    #[must_use]
    pub fn weak(&self) -> Weak<AtomicBool> {
        Arc::downgrade(&self.0)
    }
}
