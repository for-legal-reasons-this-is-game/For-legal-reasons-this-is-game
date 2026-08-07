pub mod error;
pub mod ids;
pub mod order;
pub mod price;
pub mod quantity;

pub const SCALE: u32 = 8;

pub const ONE: i64 = 100_000_000;

const _: () = assert!(ONE == 10i64.pow(SCALE));
