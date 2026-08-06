pub mod error;
pub mod ids;
pub mod price;
pub mod quantity;

// Decimal places carried by every price and quantity in the engine.
pub const SCALE: u32 = 8;

// One whole unit in minor units (10^SCALE).
pub const ONE: i64 = 100_000_000;

const _: () = assert!(ONE == 10i64.pow(SCALE));
