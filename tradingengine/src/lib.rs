pub mod book;
pub mod error;
pub mod ids;
pub mod order;
pub mod price;
pub mod quantity;
pub mod resting_order;

pub const SCALE: u32 = 8;

pub const ONE: u128 = 100_000_000;

const _: () = assert!(ONE == 10u128.pow(SCALE));
