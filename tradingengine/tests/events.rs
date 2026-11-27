use proto::execution_report::{OrderAmount, RemainingAmount};
use proto::{CancelReason, ExecType, Id, OrderStatus, Uint128};
use tradingengine::book::{Book, Fill};
use tradingengine::events::{execution_report, trade_settlement};
use tradingengine::ids::{AccountId, CommandId, OrderId, TradeId, UserId};
use tradingengine::order::Side;
use tradingengine::price::Price;
use tradingengine::quantity::BaseQuantity;
use tradingengine::quote::QuoteScale;
use tradingengine::resting_order::{RestingOrder, SettlementInfo};

// prices, base and quote all with 8 decimals, so quote = price * base / 10^8
const ONE: u128 = 100_000_000;
const FEE_ACCOUNT: u128 = 1;

/// Order `n`: holder `2_000 + n`, accounts `3_000 + n` and `4_000 + n`, command `1_000 + n`.
fn order(n: u128, side: Side, whole_price: u128, quantity: u128) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(n),
        UserId::new(2_000 + n),
        side,
        Price::from_minor_units(whole_price * ONE).expect("valid price"),
        BaseQuantity::from_minor_units(quantity),
        CommandId::new(1_000 + n),
        SettlementInfo {
            base_account: AccountId::new(3_000 + n),
            quote_account: AccountId::new(4_000 + n),
        },
    )
    .expect("valid order")
}

/// Order 1 rests, order 2 comes in and trades against it. Returns the one match.
fn one_fill(maker: RestingOrder, taker: RestingOrder) -> Fill {
    let scale = QuoteScale::new(8, 8, 8).expect("valid scale");
    let mut book = Book::new();
    book.insert(maker, scale).expect("maker rests");
    let execution = book.insert(taker, scale).expect("taker matches");
    assert_eq!(execution.fills().len(), 1);
    execution.fills()[0].clone()
}

fn id(n: u128) -> Option<Id> {
    Some(Id::from(n))
}

fn amount(n: u128) -> Option<Uint128> {
    Some(Uint128::from(n))
}

// ---- TradeSettlement ----

#[test]
fn a_buying_taker_is_the_buyer_and_the_maker_the_seller() {
    let fill = one_fill(order(1, Side::Sell, 100, 10), order(2, Side::Buy, 100, 4));
    let trade_id = TradeId::new(77);

    let settlement = trade_settlement(&fill, trade_id, Id::from(FEE_ACCOUNT));

    assert_eq!(settlement.trade_id, id(77));
    assert_eq!(settlement.price, amount(100 * ONE));
    assert_eq!(settlement.base_quantity, amount(4));
    assert_eq!(settlement.quote_quantity, amount(400));
    assert_eq!(settlement.taker_side(), proto::Side::Buy);

    assert_eq!(settlement.buyer_order_id, id(2));
    assert_eq!(settlement.buyer_account_holder, id(2_002));
    assert_eq!(settlement.buyer_base_account, id(3_002));
    assert_eq!(settlement.buyer_quote_account, id(4_002));

    assert_eq!(settlement.seller_order_id, id(1));
    assert_eq!(settlement.seller_account_holder, id(2_001));
    assert_eq!(settlement.seller_base_account, id(3_001));
    assert_eq!(settlement.seller_quote_account, id(4_001));

    assert_eq!(settlement.fee_account, id(FEE_ACCOUNT));
}

#[test]
fn a_selling_taker_is_the_seller_and_the_maker_the_buyer() {
    let fill = one_fill(order(1, Side::Buy, 100, 10), order(2, Side::Sell, 100, 4));

    let settlement = trade_settlement(&fill, TradeId::new(77), Id::from(FEE_ACCOUNT));

    assert_eq!(settlement.taker_side(), proto::Side::Sell);
    assert_eq!(settlement.buyer_order_id, id(1));
    assert_eq!(settlement.seller_order_id, id(2));
}

#[test]
fn fees_are_zero_until_fees_are_added() {
    let fill = one_fill(order(1, Side::Sell, 100, 10), order(2, Side::Buy, 100, 4));

    let settlement = trade_settlement(&fill, TradeId::new(77), Id::from(FEE_ACCOUNT));

    assert_eq!(settlement.buyer_fee, amount(0));
    assert_eq!(settlement.seller_fee, amount(0));
}

// ---- ExecutionReport ----

#[test]
fn a_new_order_reports_new_with_nothing_filled() {
    let order = order(1, Side::Buy, 100, 10);

    let report = execution_report(&order, ExecType::New, None).expect("report");

    assert_eq!(report.exec_type(), ExecType::New);
    assert_eq!(report.status(), OrderStatus::New);
    assert_eq!(report.order_id, id(1));
    assert_eq!(report.command_id, id(1_001));
    assert_eq!(report.account_holder, id(2_001));
    assert_eq!(report.side(), proto::Side::Buy);
    assert_eq!(
        report.order_amount,
        Some(OrderAmount::OrderBaseQuantity(Uint128::from(10u128)))
    );
    assert_eq!(
        report.remaining_amount,
        Some(RemainingAmount::RemainingBaseQuantity(Uint128::from(
            10u128
        )))
    );
    assert_eq!(report.filled_base_quantity, amount(0));
    assert_eq!(report.filled_quote_quantity, amount(0));
    assert_eq!(report.limit_price, amount(100 * ONE));
    assert_eq!(report.trade_id, None);
}

#[test]
fn a_trade_report_has_the_order_right_after_the_match_and_its_trade_id() {
    let fill = one_fill(order(1, Side::Sell, 100, 4), order(2, Side::Buy, 100, 10));

    let maker =
        execution_report(&fill.maker, ExecType::Trade, Some(TradeId::new(77))).expect("report");
    let taker =
        execution_report(&fill.taker, ExecType::Trade, Some(TradeId::new(77))).expect("report");

    // the maker traded out completely
    assert_eq!(maker.status(), OrderStatus::Filled);
    assert_eq!(maker.filled_base_quantity, amount(4));
    assert_eq!(maker.filled_quote_quantity, amount(400));
    assert_eq!(
        maker.remaining_amount,
        Some(RemainingAmount::RemainingBaseQuantity(Uint128::from(0u128)))
    );
    assert_eq!(maker.trade_id, id(77));

    // the taker got 4 of its 10
    assert_eq!(taker.status(), OrderStatus::PartiallyFilled);
    assert_eq!(taker.filled_base_quantity, amount(4));
    assert_eq!(
        taker.remaining_amount,
        Some(RemainingAmount::RemainingBaseQuantity(Uint128::from(6u128)))
    );
    assert_eq!(taker.trade_id, id(77));
}

#[test]
fn a_cancel_report_says_the_user_asked_for_it() {
    let mut order = order(1, Side::Buy, 100, 10);
    order.cancel().expect("an open order cancels");

    let report = execution_report(&order, ExecType::Cancelled, None).expect("report");

    assert_eq!(report.exec_type(), ExecType::Cancelled);
    assert_eq!(report.status(), OrderStatus::Cancelled);
    assert_eq!(report.cancel_reason(), CancelReason::UserRequest);
}

#[test]
fn only_a_cancel_report_has_a_cancel_reason() {
    let order = order(1, Side::Buy, 100, 10);

    let report = execution_report(&order, ExecType::New, None).expect("report");

    assert_eq!(report.cancel_reason(), CancelReason::Unspecified);
}
