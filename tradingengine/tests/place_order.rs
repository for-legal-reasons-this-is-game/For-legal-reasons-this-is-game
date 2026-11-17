use proto::place_order_request::Amount;
use proto::{
    ConfigRejectReason, Id, Market, OrderStatus, OrderType, PlaceOrderRequest, PlaceOrderResponse,
    RejectReason, SetMarketRequest, Side, TimeInForce, Timestamp, TriggerDirection, Uint128,
};
use tradingengine::engine::Engine;

// BTC-USD: base in satoshis (8), quote in cents (2), price in cents (2), so
// k = 2 + 8 - 2 = 8 and 0.5 BTC at $29,000 is 1_450_000 cents
const HALF_BTC: u128 = 50_000_000;
const PRICE_29_000: u128 = 2_900_000;

fn btc_usd(price_decimals: u32) -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    engine
}

// a valid GTC limit order. every order gets its own order, command and holder
// id from `n`, so no two orders collide and no order trades with its own holder
fn limit(n: u128, side: Side, price: u128, base_quantity: u128) -> PlaceOrderRequest {
    let mut request = PlaceOrderRequest {
        command_id: Some(Id::from(1_000 + n)),
        order_id: Some(Id::from(n)),
        market_symbol: "BTC-USD".to_string(),
        account_holder: Some(Id::from(2_000 + n)),
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

fn filled_base(response: &PlaceOrderResponse) -> u128 {
    response.filled_base_quantity.map(u128::from).unwrap_or(0)
}

fn filled_quote(response: &PlaceOrderResponse) -> u128 {
    response.filled_quote_quantity.map(u128::from).unwrap_or(0)
}

fn assert_accepted(response: &PlaceOrderResponse) {
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "rejected: {}",
        response.reject_detail
    );
}

// a rejected place must not create an order or touch the book
fn assert_rejected(engine: &mut Engine, request: PlaceOrderRequest, reason: RejectReason) {
    let before = engine.clone();
    let response = engine.place_order(request);
    assert_eq!(
        response.reject_reason(),
        reason,
        "{}",
        response.reject_detail
    );
    assert!(!response.reject_detail.is_empty());
    assert_eq!(*engine, before);
}

// ---- accepted ----

#[test]
fn an_order_that_does_not_cross_rests_as_new() {
    let mut engine = engine();

    let response = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_accepted(&response);
    assert_eq!(response.status(), OrderStatus::New);
    assert_eq!(filled_base(&response), 0);
    assert_eq!(filled_quote(&response), 0);
    let book = engine.market("BTC-USD").expect("market exists").book();
    assert_eq!(book.len(), 1);
}

#[test]
fn a_crossing_order_fills_at_the_makers_price() {
    let mut engine = engine();
    engine.place_order(limit(1, Side::Sell, PRICE_29_000, HALF_BTC));

    // the buyer would pay up to $29,100, but the maker asked $29,000
    let response = engine.place_order(limit(2, Side::Buy, 2_910_000, HALF_BTC));

    assert_accepted(&response);
    assert_eq!(response.status(), OrderStatus::Filled);
    assert_eq!(filled_base(&response), HALF_BTC);
    assert_eq!(filled_quote(&response), 1_450_000);
    assert!(
        engine
            .market("BTC-USD")
            .expect("market exists")
            .book()
            .is_empty()
    );
}

#[test]
fn a_partly_filled_order_rests_the_rest() {
    let mut engine = engine();
    engine.place_order(limit(1, Side::Sell, PRICE_29_000, 20_000_000));

    let response = engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_accepted(&response);
    assert_eq!(response.status(), OrderStatus::PartiallyFilled);
    assert_eq!(filled_base(&response), 20_000_000);
    assert_eq!(filled_quote(&response), 580_000);
    let book = engine.market("BTC-USD").expect("market exists").book();
    assert_eq!(book.len(), 1);
}

#[test]
fn the_filled_quote_is_rounded_down_per_fill() {
    let mut engine = engine();
    // at 3 cents per BTC, 0.5 BTC is 1.5 cents, which rounds down to 1
    engine.place_order(limit(1, Side::Sell, 3, HALF_BTC));
    engine.place_order(limit(2, Side::Sell, 3, HALF_BTC));

    let response = engine.place_order(limit(3, Side::Buy, 3, 2 * HALF_BTC));

    // two fills of 1 cent each, not one quote of 3 cents for the whole 1 BTC
    assert_eq!(response.status(), OrderStatus::Filled);
    assert_eq!(filled_quote(&response), 2);
}

#[test]
fn the_command_id_comes_back_accepted_or_not() {
    let mut engine = engine();

    let accepted = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let mut unknown = limit(2, Side::Buy, PRICE_29_000, HALF_BTC);
    unknown.market_symbol = "ETH-USD".to_string();
    let rejected = engine.place_order(unknown);

    assert_eq!(accepted.command_id, Some(Id::from(1_001u128)));
    assert_eq!(rejected.command_id, Some(Id::from(1_002u128)));
}

// ---- rejected, in the order of the contract's validation table ----

#[test]
fn an_unknown_market_is_rejected() {
    let mut engine = engine();
    let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    request.market_symbol = "ETH-USD".to_string();

    assert_rejected(&mut engine, request, RejectReason::UnknownMarket);
}

#[test]
fn a_missing_id_is_bad_order_params() {
    let mut engine = engine();
    let missing: [fn(&mut PlaceOrderRequest); 5] = [
        |r| r.command_id = None,
        |r| r.order_id = None,
        |r| r.account_holder = None,
        |r| r.base_account = None,
        |r| r.quote_account = None,
    ];

    for clear in missing {
        let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
        clear(&mut request);
        assert_rejected(&mut engine, request, RejectReason::BadOrderParams);
    }
}

#[test]
fn an_unset_or_unknown_enum_is_bad_order_params() {
    let mut engine = engine();

    let mut no_side = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    no_side.side = 0;
    assert_rejected(&mut engine, no_side, RejectReason::BadOrderParams);

    // a number the proto does not know reads as UNSPECIFIED
    let mut unknown_type = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    unknown_type.r#type = 99;
    assert_rejected(&mut engine, unknown_type, RejectReason::BadOrderParams);

    let mut no_time_in_force = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    no_time_in_force.time_in_force = 0;
    assert_rejected(&mut engine, no_time_in_force, RejectReason::BadOrderParams);
}

#[test]
fn a_missing_or_wrong_amount_arm_is_bad_order_params() {
    let mut engine = engine();

    let mut no_amount = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    no_amount.amount = None;
    assert_rejected(&mut engine, no_amount, RejectReason::BadOrderParams);

    // only market buys say how much quote to spend
    let mut quote_on_a_limit = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    quote_on_a_limit.amount = Some(Amount::QuoteQuantity(Uint128::from(1_000u128)));
    assert_rejected(&mut engine, quote_on_a_limit, RejectReason::BadOrderParams);
}

#[test]
fn a_zero_quantity_is_bad_quantity() {
    let mut engine = engine();

    assert_rejected(
        &mut engine,
        limit(1, Side::Buy, PRICE_29_000, 0),
        RejectReason::BadQuantity,
    );
}

#[test]
fn a_zero_price_is_bad_price() {
    let mut engine = engine();

    assert_rejected(
        &mut engine,
        limit(1, Side::Buy, 0, HALF_BTC),
        RejectReason::BadPrice,
    );
}

#[test]
fn a_limit_order_without_a_limit_price_is_bad_order_params() {
    let mut engine = engine();
    let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    request.limit_price = None;

    assert_rejected(&mut engine, request, RejectReason::BadOrderParams);
}

#[test]
fn stop_fields_on_a_limit_order_are_bad_order_params() {
    let mut engine = engine();

    let mut trigger = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    trigger.trigger_price = Some(Uint128::from(PRICE_29_000));
    assert_rejected(&mut engine, trigger, RejectReason::BadOrderParams);

    let mut direction = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    direction.set_trigger_direction(TriggerDirection::LastAtOrAbove);
    assert_rejected(&mut engine, direction, RejectReason::BadOrderParams);
}

#[test]
fn expire_time_must_go_with_gtd_and_only_gtd() {
    let mut engine = engine();

    let mut gtd_without_time = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    gtd_without_time.set_time_in_force(TimeInForce::Gtd);
    assert_rejected(&mut engine, gtd_without_time, RejectReason::BadOrderParams);

    let mut gtc_with_time = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    gtc_with_time.expire_time = Some(Timestamp {
        seconds: 1_900_000_000,
        nanos: 0,
    });
    assert_rejected(&mut engine, gtc_with_time, RejectReason::BadOrderParams);
}

#[test]
fn a_protection_price_on_a_limit_order_is_bad_order_params() {
    let mut engine = engine();
    let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    request.protection_price = Some(Uint128::from(PRICE_29_000));

    assert_rejected(&mut engine, request, RejectReason::BadOrderParams);
}

#[test]
fn a_quote_that_does_not_fit_in_128_bits_is_bad_quantity() {
    let mut engine = engine();

    assert_rejected(
        &mut engine,
        limit(1, Side::Buy, u128::MAX, 1_000_000_000),
        RejectReason::BadQuantity,
    );
}

#[test]
fn a_sell_that_would_overflow_at_the_best_bid_is_rejected_before_matching() {
    let mut engine = engine();
    // 1 satoshi at a huge price fits on its own
    let response = engine.place_order(limit(1, Side::Buy, 10u128.pow(30), 1));
    assert_accepted(&response);

    // the sell's own limit is tiny, but it would fill at the 10^30 bid:
    // 10^30 * 10^20 / 10^8 does not fit, so it must be rejected untouched
    assert_rejected(
        &mut engine,
        limit(2, Side::Sell, 1, 10u128.pow(20)),
        RejectReason::BadQuantity,
    );
}

// ---- valid, but not built yet ----

#[test]
fn a_market_order_is_unsupported() {
    let mut engine = engine();
    let mut request = limit(1, Side::Sell, PRICE_29_000, HALF_BTC);
    request.set_type(OrderType::Market);
    request.limit_price = None;

    assert_rejected(&mut engine, request, RejectReason::Unsupported);
}

#[test]
fn a_stop_limit_order_is_unsupported() {
    let mut engine = engine();
    let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    request.set_type(OrderType::StopLimit);
    request.trigger_price = Some(Uint128::from(PRICE_29_000));
    request.set_trigger_direction(TriggerDirection::LastAtOrAbove);

    assert_rejected(&mut engine, request, RejectReason::Unsupported);
}

#[test]
fn ioc_fok_and_gtd_are_unsupported() {
    let mut engine = engine();

    for time_in_force in [TimeInForce::Ioc, TimeInForce::Fok] {
        let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
        request.set_time_in_force(time_in_force);
        assert_rejected(&mut engine, request, RejectReason::Unsupported);
    }

    let mut gtd = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    gtd.set_time_in_force(TimeInForce::Gtd);
    gtd.expire_time = Some(Timestamp {
        seconds: 1_900_000_000,
        nanos: 0,
    });
    assert_rejected(&mut engine, gtd, RejectReason::Unsupported);
}

// ---- other rules ----

#[test]
fn a_duplicate_order_id_is_rejected_and_the_book_kept() {
    let mut engine = engine();
    engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    // a new command reusing a live order_id is a bad request, not a crash
    let mut again = limit(1, Side::Buy, 2_800_000, HALF_BTC);
    again.command_id = Some(Id::from(9_999u128));
    assert_rejected(&mut engine, again, RejectReason::BadOrderParams);
}

#[test]
fn open_orders_stop_a_market_from_changing_its_decimals() {
    // the HAS_OPEN_ORDERS rule from SetMarket, testable now that an order can rest
    let mut engine = engine();
    engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let response = engine.set_market(SetMarketRequest {
        market: Some(btc_usd(3)),
    });

    assert_eq!(response.reject_reason(), ConfigRejectReason::HasOpenOrders);
    let market = engine.market("BTC-USD").expect("market exists");
    assert_eq!(market.market().price_decimals, 2);
}

#[test]
fn open_orders_still_allow_a_new_fee_account() {
    let mut engine = engine();
    engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let mut new_fee_account = btc_usd(2);
    new_fee_account.fee_account = Some(Id::from(2u128));
    let response = engine.set_market(SetMarketRequest {
        market: Some(new_fee_account),
    });

    assert_eq!(response.reject_reason(), ConfigRejectReason::Unspecified);
    let market = engine.market("BTC-USD").expect("market exists");
    assert_eq!(market.book().len(), 1);
}
