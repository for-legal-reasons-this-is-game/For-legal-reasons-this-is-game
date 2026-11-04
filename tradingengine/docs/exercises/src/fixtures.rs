use proto::place_order_request::Amount;
use proto::{Id, Market, OrderType, PlaceOrderRequest, TimeInForce, Uint128};

pub const BTC_USD: &str = "BTC-USD";

// 1 BTC in satoshis
pub const BTC: u128 = 100_000_000;

// the running example from DATA_MODEL.md: satoshis, cents, price in cents, k = 8
pub fn btc_usd() -> Market {
    market(BTC_USD)
}

pub fn market(symbol: &str) -> Market {
    Market {
        symbol: symbol.to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals: 2,
        fee_account: Some(Id::from(1u128)),
    }
}

// a price in dollars, written in cents
pub const fn dollars(amount: u128) -> u128 {
    amount * 100
}

// a valid limit GTC request on BTC-USD. Change fields with struct update
// syntax: PlaceOrderRequest { limit_price: None, ..limit_request(...) }
pub fn limit_request(side: proto::Side, price: u128, base: u128) -> PlaceOrderRequest {
    PlaceOrderRequest {
        command_id: Some(Id::from(100u128)),
        order_id: Some(Id::from(200u128)),
        market_symbol: BTC_USD.to_string(),
        account_holder: Some(Id::from(300u128)),
        base_account: Some(Id::from(400u128)),
        quote_account: Some(Id::from(500u128)),
        side: side as i32,
        r#type: OrderType::Limit as i32,
        time_in_force: TimeInForce::Gtc as i32,
        amount: Some(Amount::BaseQuantity(Uint128::from(base))),
        limit_price: Some(Uint128::from(price)),
        maker_fee_bps: 5,
        taker_fee_bps: 10,
        ..PlaceOrderRequest::default()
    }
}
