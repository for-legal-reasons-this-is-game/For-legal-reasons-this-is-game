use super::*;
use crate::ONE;

#[test]
fn rejects_zero() {
    assert_eq!(
        Price::from_minor_units(0),
        Err(EngineError::PriceNotPositive)
    );
}

#[test]
fn rejects_negative() {
    assert_eq!(
        Price::from_minor_units(-1),
        Err(EngineError::PriceNotPositive)
    );
}

#[test]
fn accepts_the_smallest_positive_value() {
    let price = Price::from_minor_units(1).expect("one minor unit is a valid price");
    assert_eq!(price.minor_units(), 1);
}

#[test]
fn minor_units_round_trips() {
    // 29_000.0 expressed in minor units
    let raw = 29_000 * ONE;
    let price = Price::from_minor_units(raw).expect("valid price");
    assert_eq!(price.minor_units(), raw);
}

#[test]
fn higher_price_compares_above_lower() {
    let low = Price::from_minor_units(29_000 * ONE).expect("valid price");
    let high = Price::from_minor_units(29_001 * ONE).expect("valid price");

    assert!(high > low);
    assert!(low < high);
    assert_ne!(low, high);
}

#[test]
fn prices_sort_themselves_in_a_btreemap() {
    // this is the property the order book is built on: a BTreeMap keyed by
    // Price keeps the levels sorted for free, because Price is Ord
    let mut levels = std::collections::BTreeMap::new();
    for whole in [29_002, 29_000, 29_001] {
        let price = Price::from_minor_units(whole * ONE).expect("valid price");
        levels.insert(price, whole);
    }

    let ascending: Vec<i64> = levels.values().copied().collect();
    assert_eq!(ascending, vec![29_000, 29_001, 29_002]);

    // asks read best-first from the front, bids best-first from the back
    assert_eq!(
        levels.keys().next().copied(),
        Price::from_minor_units(29_000 * ONE).ok()
    );
    assert_eq!(
        levels.keys().next_back().copied(),
        Price::from_minor_units(29_002 * ONE).ok()
    );
}
