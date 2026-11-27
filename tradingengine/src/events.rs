//! builds the event messages from `proto/events.proto` out of engine data
//!
//! this only builds them. the `sequence`, the `engine_time` and sending them
//! happen somewhere else

use proto::execution_report::{OrderAmount, RemainingAmount};
use proto::{ExecType, ExecutionReport, Id, OrderType, TimeInForce, TradeSettlement, Uint128};

use crate::book::Fill;
use crate::error::Result;
use crate::ids::TradeId;
use crate::order::Side;
use crate::resting_order::RestingOrder;
use crate::wire::{order_status, side_to_proto};

/// the `TradeSettlement` for one match. it has both sides so the backend can
/// settle the trade without looking anything up
///
/// fees are 0 for now, they come after the evaluation
pub fn trade_settlement(fill: &Fill, trade_id: TradeId, fee_account: Id) -> TradeSettlement {
    let (buyer, seller) = match fill.taker.side() {
        Side::Buy => (&fill.taker, &fill.maker),
        Side::Sell => (&fill.maker, &fill.taker),
    };
    let no_fee = Some(Uint128::from(0u128));

    let mut settlement = TradeSettlement {
        trade_id: Some(trade_id.into()),
        price: Some(fill.price.into()),
        base_quantity: Some(fill.quantity.into()),
        quote_quantity: Some(fill.quote.into()),

        buyer_order_id: Some(buyer.id().into()),
        buyer_account_holder: Some(buyer.user_id().into()),
        buyer_base_account: Some(buyer.settlement().base_account.into()),
        buyer_quote_account: Some(buyer.settlement().quote_account.into()),
        buyer_fee: no_fee,

        seller_order_id: Some(seller.id().into()),
        seller_account_holder: Some(seller.user_id().into()),
        seller_base_account: Some(seller.settlement().base_account.into()),
        seller_quote_account: Some(seller.settlement().quote_account.into()),
        seller_fee: no_fee,

        fee_account: Some(fee_account),
        ..Default::default()
    };
    settlement.set_taker_side(side_to_proto(fill.taker.side()));
    settlement
}

/// the `ExecutionReport` for `order` as it is right now. every report has the full
/// totals of the order so the backend just overwrites what it had
///
/// `trade_id` is only for `ExecType::Trade`. `command_id` is the command that placed
/// the order. only GTC limit orders exist for now and the only cancel is the users own
pub fn execution_report(
    order: &RestingOrder,
    exec_type: ExecType,
    trade_id: Option<TradeId>,
) -> Result<ExecutionReport> {
    let filled_base = order.qty_total().checked_sub(order.qty_remaining())?;

    let mut report = ExecutionReport {
        order_id: Some(order.id().into()),
        command_id: Some(order.command_id().into()),
        account_holder: Some(order.user_id().into()),
        order_amount: Some(OrderAmount::OrderBaseQuantity(order.qty_total().into())),
        remaining_amount: Some(RemainingAmount::RemainingBaseQuantity(
            order.qty_remaining().into(),
        )),
        filled_base_quantity: Some(filled_base.into()),
        filled_quote_quantity: Some(order.filled_quote().into()),
        limit_price: Some(order.price().into()),
        trade_id: trade_id.map(Id::from),
        ..Default::default()
    };
    report.set_exec_type(exec_type);
    report.set_status(order_status(order.status()));
    report.set_side(side_to_proto(order.side()));
    report.set_type(OrderType::Limit);
    report.set_time_in_force(TimeInForce::Gtc);
    if exec_type == ExecType::Cancelled {
        report.set_cancel_reason(proto::CancelReason::UserRequest);
    }
    Ok(report)
}
