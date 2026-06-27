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
