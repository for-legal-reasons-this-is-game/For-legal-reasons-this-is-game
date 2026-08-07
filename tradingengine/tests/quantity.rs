use tradingengine::error::EngineError;
use tradingengine::quantity::Quantity;

fn qty(minor_units: i64) -> Quantity {
    Quantity::from_minor_units(minor_units).expect("valid quantity")
}

#[test]
fn accepts_zero() {
    // unlike Price, zero is legal: "filled so far" starts here
    let quantity = qty(0);
    assert_eq!(quantity, Quantity::ZERO);
    assert!(quantity.is_zero());
}

#[test]
fn accepts_positive() {
    assert_eq!(qty(5).minor_units(), 5);
}

#[test]
fn rejects_negative() {
    assert_eq!(
        Quantity::from_minor_units(-1),
        Err(EngineError::QuantityNegative)
    );
}

#[test]
fn require_positive_rejects_zero() {
    assert_eq!(
        Quantity::ZERO.require_positive(),
        Err(EngineError::QuantityNotPositive)
    );
}

#[test]
fn require_positive_passes_a_positive_value_through() {
    assert_eq!(qty(1).require_positive(), Ok(qty(1)));
}

#[test]
fn checked_add_sums() {
    assert_eq!(qty(3).checked_add(qty(4)), Ok(qty(7)));
}

#[test]
fn checked_add_reports_overflow() {
    let max = qty(i64::MAX);
    assert_eq!(max.checked_add(qty(1)), Err(EngineError::Overflow));
}

#[test]
fn checked_sub_subtracts() {
    // the direction matters: this caught an earlier version that added
    assert_eq!(qty(10).checked_sub(qty(3)), Ok(qty(7)));
}

#[test]
fn checked_sub_can_reach_exactly_zero() {
    assert_eq!(qty(5).checked_sub(qty(5)), Ok(Quantity::ZERO));
}

#[test]
fn checked_sub_refuses_to_go_negative() {
    // an i64 would happily wrap here; the type must not
    assert_eq!(
        qty(3).checked_sub(qty(10)),
        Err(EngineError::QuantityNegative)
    );
}

#[test]
fn larger_quantity_compares_above_smaller() {
    assert!(qty(10) > qty(3));
    assert!(Quantity::ZERO < qty(1));
}
