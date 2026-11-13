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
/// modify command".
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

    match market.book().get_by_id(order_id) {
        Some(resting_order) if resting_order.user_id() != account_holder => {
            // ERR #4 from contract
            Err(EngineError::NotOrderOwner)
        }
        Some(_) => Ok((market, order_id)),
        // ERR #2 and ERR #3 from contract (the order has ended) can't be told apart yet: the book
        // forgets an order once it is filled or cancelled, so an ended order lands here too.
        // TODO(mehras): the contract wants ORDER_NOT_WORKING for an ended order, not
        // UNKNOWN_ORDER. Settle where ended orders are remembered, then check them here
        None => Err(EngineError::OrderNotFound),
    }
}
