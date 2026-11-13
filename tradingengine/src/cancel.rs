use std::collections::HashMap;

use proto::CancelOrderRequest;

use crate::{
    error::{EngineError, Result},
    ids::{OrderId, UserId},
    market::MarketInfo,
    wire::required_id,
};

/// Checks a [`CancelOrderRequest`] and finds the market of the order to cancel.
///
/// Returns that market and the order's id once the order is in the market's book and
/// `account_holder` owns it. The checks follow `proto/how_to_use.md`, "Validating a cancel or
/// modify command". An order that has already ended is rejected with its final status, but only
/// to its owner: anyone else gets `NOT_ORDER_OWNER`, so a cancel tells them nothing about it.
pub fn parse_cancel_order(
    request: CancelOrderRequest,
    markets: &mut HashMap<String, MarketInfo>,
) -> Result<(&mut MarketInfo, OrderId)> {
    if request.market_symbol.is_empty() {
        // ERR #1 from contract
        return Err(EngineError::UnknownMarket);
    }
    if request.order_id.is_none() {
        return Err(EngineError::RequiredFieldAbsent);
    }
    if request.command_id.is_none() {
        return Err(EngineError::BadOrderParams("command_id is empty"));
    }
    if request.account_holder.is_none() {
        return Err(EngineError::BadOrderParams("account_holder is empty"));
    }

    let market = markets
        .get_mut(&request.market_symbol)
        // ERR #1 from contract
        .ok_or(EngineError::UnknownMarket)?;
    let order_id = OrderId::new(required_id(request.order_id)?);
    let account_holder = UserId::new(required_id(request.account_holder)?);

    match (
        market.book().get_by_id(order_id),
        market.ended().get(order_id),
    ) {
        // ERR #3 from contract, for an order in the book
        (Some(resting_order), _) if resting_order.user_id() != account_holder => {
            Err(EngineError::NotOrderOwner)
        }
        (Some(_), _) => Ok((market, order_id)),
        // ERR #3 from contract, for an order that has ended
        (None, Some(ended)) if ended.owner != account_holder => Err(EngineError::NotOrderOwner),
        // ERR #4 from contract: ORDER_NOT_WORKING, with how the order ended
        (None, Some(ended)) => Err(EngineError::OrderNotLive(ended.status)),
        // ERR #2 from contract: never placed, rejected when it was placed, or ended so long ago
        // that it was forgotten
        (None, None) => Err(EngineError::OrderNotFound),
    }
}
