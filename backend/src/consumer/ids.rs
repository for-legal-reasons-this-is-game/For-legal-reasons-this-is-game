use sha2::{Digest, Sha256};

// Transfer ids must be the same every time the same event is replayed. The high 64 bits keep the
// time prefix of the source id (tb::id() / UUIDv7) so tigerbeetle keeps its ordering; the low 64
// bits come from a hash. `source + leg` would not be safe: ids inside one millisecond are
// consecutive, so leg 2 of one trade could equal leg 1 of the next.
fn derive(tag: &[u8], source: u128, leg: u8) -> u128 {
    let mut hasher = Sha256::new();
    hasher.update(tag);
    hasher.update(source.to_be_bytes());
    hasher.update([leg]);
    let digest = hasher.finalize();
    let mut low = [0u8; 8];
    low.copy_from_slice(&digest[..8]);
    let id = (source & (u64::MAX as u128) << 64) | u64::from_be_bytes(low) as u128;
    // tigerbeetle rejects 0 and u128::MAX as ids
    if id == 0 || id == u128::MAX { 1 } else { id }
}

// id of one leg (1..=4) of the transfer chain that settles a trade
pub fn trade_leg_transfer_id(trade_id: u128, leg: u8) -> u128 {
    derive(b"trade-leg", trade_id, leg)
}

// id of the transfer that returns what is left in an order's hold account
pub fn refund_transfer_id(order_id: u128) -> u128 {
    derive(b"refund", order_id, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legs_of_one_trade_have_distinct_ids() {
        let t = 0x0123_4567_89ab_cdef_0011_2233_4455_6677u128;
        let ids: Vec<u128> = (1..=4).map(|leg| trade_leg_transfer_id(t, leg)).collect();
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                assert_ne!(ids[i], ids[j]);
            }
        }
    }

    #[test]
    fn consecutive_trade_ids_do_not_collide_across_legs() {
        // ids inside one millisecond are consecutive; leg 2 of A must not equal leg 1 of A+1
        let a = 0x0123_4567_89ab_cdef_0000_0000_0000_0010u128;
        assert_ne!(trade_leg_transfer_id(a, 2), trade_leg_transfer_id(a + 1, 1));
    }

    #[test]
    fn ids_are_deterministic() {
        assert_eq!(trade_leg_transfer_id(42, 3), trade_leg_transfer_id(42, 3));
        assert_eq!(refund_transfer_id(42), refund_transfer_id(42));
    }

    #[test]
    fn refund_id_differs_from_trade_leg_id() {
        assert_ne!(refund_transfer_id(7), trade_leg_transfer_id(7, 1));
    }

    #[test]
    fn id_keeps_the_time_prefix_of_its_source() {
        let t = 0xAAAA_BBBB_CCCC_DDDD_0000_0000_0000_0001u128;
        assert_eq!(trade_leg_transfer_id(t, 1) >> 64, t >> 64);
        assert_eq!(refund_transfer_id(t) >> 64, t >> 64);
    }

    #[test]
    fn ids_are_never_zero_or_max() {
        for src in [0u128, 1, u128::MAX] {
            for id in [trade_leg_transfer_id(src, 1), refund_transfer_id(src)] {
                assert_ne!(id, 0);
                assert_ne!(id, u128::MAX);
            }
        }
    }
}
