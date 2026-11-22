//! Helpers the engine's tests share: the BTC-USD market, and orders and cancels built from one
//! number `n`.
//!
//! Every file in `tests/` is its own crate and uses only some of these, so the ones a file
//! doesn't use are allowed here instead of warning in that file.
#![allow(dead_code)]

use proto::place_order_request::Amount;
use proto::{
    CancelOrderRequest, Id, Market, OrderType, PlaceOrderRequest, PlaceOrderResponse, RejectReason,
    SetMarketRequest, Side, TimeInForce, Uint128,
};
use tradingengine::engine::Engine;
use tradingengine::ids::OrderId;

// BTC-USD: base in satoshis (8), quote in cents (2), price in cents (2), so
// k = 2 + 8 - 2 = 8 and 0.5 BTC at $29,000 is 1_450_000 cents
pub const HALF_BTC: u128 = 50_000_000;
pub const PRICE_29_000: u128 = 2_900_000;

/// A BTC-USD market in satoshis and cents, with prices written with `price_decimals` decimals.
pub fn btc_usd(price_decimals: u32) -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

/// An engine with the BTC-USD market set, with prices in cents.
pub fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    engine
}

/// Like [`engine`], with room for `capacity` command cache entries.
pub fn engine_with_cache(capacity: usize) -> Engine {
    let mut engine = Engine::with_cache_capacity(capacity);
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    engine
}

/// The account holder that places order `n` with [`limit`].
pub fn owner(n: u128) -> u128 {
    2_000 + n
}

/// A valid GTC limit order. Every order gets its own order, command and holder id from `n`,
/// so no two orders collide and no order trades with its own holder. Order `n` has
/// `command_id` `1_000 + n`.
pub fn limit(n: u128, side: Side, price: u128, base_quantity: u128) -> PlaceOrderRequest {
    let mut request = PlaceOrderRequest {
        command_id: Some(Id::from(1_000 + n)),
        order_id: Some(Id::from(n)),
        market_symbol: "BTC-USD".to_string(),
        account_holder: Some(Id::from(owner(n))),
        base_account: Some(Id::from(3_000 + n)),
        quote_account: Some(Id::from(4_000 + n)),
        amount: Some(Amount::BaseQuantity(Uint128::from(base_quantity))),
        limit_price: Some(Uint128::from(price)),
        maker_fee_bps: 5,
        taker_fee_bps: 10,
        ..PlaceOrderRequest::default()
    };
    request.set_side(side);
    request.set_type(OrderType::Limit);
    request.set_time_in_force(TimeInForce::Gtc);
    request
}

/// A cancel of order `n` by its owner.
///
/// Its `command_id` (`5_000 + n`) differs from the place's (`1_000 + n`). The engine answers a
/// `command_id` it has seen before from its command cache, so a cancel that reused the place's
/// id would get DUPLICATE_COMMAND.
pub fn cancel(n: u128) -> CancelOrderRequest {
    CancelOrderRequest {
        command_id: Some(Id::from(5_000 + n)),
        order_id: Some(Id::from(n)),
        account_holder: Some(Id::from(owner(n))),
        market_symbol: "BTC-USD".to_string(),
    }
}

/// Places an order and checks that it was accepted, so a broken place fails here and not
/// later in the test.
pub fn place(engine: &mut Engine, request: PlaceOrderRequest) {
    let response = engine.place_order(request);
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "place rejected: {}",
        response.reject_detail
    );
}

/// Whether order `n` is in the BTC-USD book.
pub fn in_book(engine: &Engine, n: u128) -> bool {
    engine
        .market("BTC-USD")
        .expect("market exists")
        .book()
        .get_by_id(OrderId::new(n))
        .is_some()
}

/// How much base a place filled.
pub fn filled_base(response: &PlaceOrderResponse) -> u128 {
    response.filled_base_quantity.map(u128::from).unwrap_or(0)
}

/// How much quote a place filled.
pub fn filled_quote(response: &PlaceOrderResponse) -> u128 {
    response.filled_quote_quantity.map(u128::from).unwrap_or(0)
}

/// Checks that a place was accepted.
pub fn assert_accepted(response: &PlaceOrderResponse) {
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "rejected: {}",
        response.reject_detail
    );
}

/// Sends a place and checks that it was rejected with `reason`: it says why, and it didn't
/// create an order or touch the book.
pub fn assert_rejected(engine: &mut Engine, request: PlaceOrderRequest, reason: RejectReason) {
    let before = engine.markets().clone();
    let response = engine.place_order(request);
    assert_eq!(
        response.reject_reason(),
        reason,
        "{}",
        response.reject_detail
    );
    assert!(!response.reject_detail.is_empty());
    assert_eq!(*engine.markets(), before);
}
