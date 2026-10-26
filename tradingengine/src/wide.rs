use primitive_types::U256;

// a * b / d rounded down. The product can need up to 256 bits, so it is done in U256.
// None if d is 0 or the result does not fit back into a u128.
pub(crate) fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }

    // Two u128s multiplied always fit in 256 bits, so this cannot overflow.
    let product = U256::from(a) * U256::from(b);
    let quotient = product / U256::from(d);

    u128::try_from(quotient).ok()
}
