const LOW_64_BITS: u128 = u64::MAX as u128;

fn multiply_to_256_bits(a: u128, b: u128) -> (u128, u128) {
    let a_high = a >> 64;
    let a_low = a & LOW_64_BITS;
    let b_high = b >> 64;
    let b_low = b & LOW_64_BITS;

    let low_times_low = a_low * b_low;
    let low_times_high = a_low * b_high;
    let high_times_low = a_high * b_low;
    let high_times_high = a_high * b_high;

    // Add up the middle column. It is three numbers below 2^64, so it fits.
    let middle =
        (low_times_low >> 64) + (low_times_high & LOW_64_BITS) + (high_times_low & LOW_64_BITS);

    let low = (low_times_low & LOW_64_BITS) | (middle << 64);
    let high = high_times_high + (low_times_high >> 64) + (high_times_low >> 64) + (middle >> 64);

    (high, low)
}

// a * b / d: NONE if d is 0

pub(crate) fn mul_div_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }

    let (high, low) = multiply_to_256_bits(a, b);

    // If the high half is already d or more, the answer needs more than 128 bits.
    if high >= d {
        return None;
    }

    // Easy case: the product fits in 128 bits, so normal division works.
    if high == 0 {
        return Some(low / d);
    }

    let mut remainder = high;
    let mut quotient: u128 = 0;

    for bit_index in (0..128).rev() {
        let top_bit_was_set = (remainder >> 127) == 1;

        let next_bit = (low >> bit_index) & 1;
        remainder = (remainder << 1) | next_bit;
        quotient <<= 1;

        if top_bit_was_set || remainder >= d {
            remainder = remainder.wrapping_sub(d);
            quotient |= 1;
        }
    }

    Some(quotient)
}
