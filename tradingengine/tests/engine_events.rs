mod common;

use common::{HALF_BTC, PRICE_29_000, cancel, engine, limit};
use proto::event_item::Body;
use proto::{EventItem, PlaceOrderRequest, Side};
use tradingengine::engine::Engine;

const QUARTER_BTC: u128 = 25_000_000;
const PRICE_28_000: u128 = 2_800_000;

/// places the order, checks it was accepted and returns its events
fn place(engine: &mut Engine, request: PlaceOrderRequest) -> Vec<EventItem> {
    let (response, events) = engine.place_order_with_events(request);
    assert_eq!(response.reject_reason(), proto::RejectReason::Unspecified);
    events
}

/// each event in short: "settlement", or the report's exec type and order id
fn describe(events: &[EventItem]) -> Vec<String> {
    events
        .iter()
        .map(
            |item| match item.body.as_ref().expect("every event has a body") {
                Body::TradeSettlement(_) => "settlement".to_string(),
                Body::ExecutionReport(report) => {
                    let order = u128::from(report.order_id.expect("report has an order id"));
                    format!("{:?} {order}", report.exec_type())
                }
            },
        )
        .collect()
}

fn sequences(events: &[EventItem]) -> Vec<u64> {
    events.iter().map(|item| item.sequence).collect()
}

/// the trade id of every settlement and trade report, in order
fn trade_ids(events: &[EventItem]) -> Vec<u128> {
    events
        .iter()
        .filter_map(|item| match item.body.as_ref()? {
            Body::TradeSettlement(settlement) => settlement.trade_id,
            Body::ExecutionReport(report) => report.trade_id,
        })
        .map(u128::from)
        .collect()
}

// ---- place ----

#[test]
fn a_resting_order_gets_one_new_report() {
    let mut engine = engine();

    let events = place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(describe(&events), ["New 1"]);
    assert_eq!(sequences(&events), [1]);
    assert_eq!(events[0].market_symbol, "BTC-USD");
    assert!(events[0].engine_time.is_some());
}

#[test]
fn a_match_is_new_then_settlement_then_buyer_then_seller() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));

    // order 2 buys, so it is the buyer and the resting order 1 the seller
    let events = place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(
        describe(&events),
        ["New 2", "settlement", "Trade 2", "Trade 1"]
    );
    assert_eq!(sequences(&events), [2, 3, 4, 5]);
}

#[test]
fn a_selling_taker_reports_after_the_buying_maker() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let events = place(&mut engine, limit(2, Side::Sell, PRICE_29_000, HALF_BTC));

    assert_eq!(
        describe(&events),
        ["New 2", "settlement", "Trade 1", "Trade 2"]
    );
}

#[test]
fn a_settlement_and_its_two_reports_share_one_trade_id() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));

    let events = place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    let ids = trade_ids(&events);
    assert_eq!(ids.len(), 3);
    assert!(ids.iter().all(|id| *id == ids[0]));
}

#[test]
fn two_matches_get_their_own_trade_ids_in_order() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_28_000, QUARTER_BTC));
    place(&mut engine, limit(2, Side::Sell, PRICE_29_000, QUARTER_BTC));

    let events = place(&mut engine, limit(3, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(
        describe(&events),
        [
            "New 3",
            "settlement",
            "Trade 3",
            "Trade 1",
            "settlement",
            "Trade 3",
            "Trade 2"
        ]
    );
    let ids = trade_ids(&events);
    // first match, then second match: each settlement and its reports share an id
    assert!(ids[..3].iter().all(|id| *id == ids[0]));
    assert!(ids[3..].iter().all(|id| *id == ids[3]));
    assert!(ids[0] < ids[3]);
}

#[test]
fn every_event_of_one_command_has_the_same_time() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));

    let events = place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert!(
        events
            .iter()
            .all(|item| item.engine_time == events[0].engine_time)
    );
}

// ---- cancel ----

#[test]
fn a_cancel_gets_one_cancelled_report() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let (_, events) = engine.cancel_order_with_events(cancel(1));

    assert_eq!(describe(&events), ["Cancelled 1"]);
    assert_eq!(sequences(&events), [2]);
}

// ---- no events ----

#[test]
fn a_rejected_order_has_no_events_and_leaves_no_gap() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let unknown_market = PlaceOrderRequest {
        market_symbol: "ETH-USD".to_string(),
        ..limit(2, Side::Buy, PRICE_29_000, HALF_BTC)
    };
    let (_, events) = engine.place_order_with_events(unknown_market);
    assert!(events.is_empty());

    let events = place(&mut engine, limit(3, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_eq!(sequences(&events), [2]);
}

#[test]
fn a_retried_order_has_no_events_the_second_time() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let events = place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    assert!(events.is_empty());
}

#[test]
fn a_rejected_cancel_has_no_events() {
    let mut engine = engine();

    let (_, events) = engine.cancel_order_with_events(cancel(1));

    assert!(events.is_empty());
}
