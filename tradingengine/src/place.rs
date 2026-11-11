use proto::place_order_request::Amount;
use proto::{OrderType, PlaceOrderRequest, TimeInForce, TriggerDirection, Uint128};

use crate::error::{EngineError, Result};
use crate::ids::{CommandId, OrderId, UserId};
use crate::price::Price;
use crate::resting_order::RestingOrder;
use crate::wire::{base_quantity_from_proto, required, required_id, side_from_proto};

/// limit and stop limit orders these are the ones with a `limit_price`
fn is_limit_type(order_type: OrderType) -> bool {
    match order_type {
        OrderType::Limit | OrderType::StopLimit => true,
        OrderType::Market | OrderType::StopMarket => false,
        OrderType::Unspecified => false,
    }
}

/// market and stop market orders they take any price so they have no `limit_price`
fn is_market_type(order_type: OrderType) -> bool {
    match order_type {
        OrderType::Market | OrderType::StopMarket => true,
        OrderType::Limit | OrderType::StopLimit => false,
        OrderType::Unspecified => false,
    }
}

/// stop orders wait for the `trigger_price` before they trade
fn is_stop_type(order_type: OrderType) -> bool {
    match order_type {
        OrderType::StopLimit | OrderType::StopMarket => true,
        OrderType::Limit | OrderType::Market => false,
        OrderType::Unspecified => false,
    }
}

/// true for a market buy because it says how much quote to spend (USD)
///
/// every other order says how much base it trades (BTC)
fn wants_quote(order_type: OrderType, side: proto::Side) -> bool {
    match (is_market_type(order_type), side) {
        (true, proto::Side::Buy) => true,
        (true, proto::Side::Sell) => false,
        (false, _) => false,
        (true, proto::Side::Unspecified) => false,
    }
}

/// true if the price was sent as zero. a price that wasnt sent is not zero
fn is_zero(price: Option<Uint128>) -> bool {
    match price {
        Some(price) => u128::from(price) == 0,
        None => false,
    }
}

/// checks a place request against the "Validating a place command" table in
/// `proto/how_to_use.md`
///
/// the checks are in the same order as the table so the first mistake decides the
/// reject reason. the engine checks the market before this and the quote overflow
/// check needs the markets scale so its in [`MarketInfo::place`]
///
/// only GTC limit orders work for now. a valid order of any other kind gets
/// `UNSUPPORTED`
///
/// [`MarketInfo::place`]: crate::market::MarketInfo::place
fn check_place_order(request: &PlaceOrderRequest) -> Result<()> {
    // ids. the two accounts arent stored yet they are for the TradeSettlement
    // but an order without them can never be setteled
    if request.command_id.is_none() {
        return Err(EngineError::BadOrderParams("command_id is missing"));
    }
    if request.order_id.is_none() {
        return Err(EngineError::BadOrderParams("order_id is missing"));
    }
    if request.account_holder.is_none() {
        return Err(EngineError::BadOrderParams("account_holder is missing"));
    }
    if request.base_account.is_none() {
        return Err(EngineError::BadOrderParams("base_account is missing"));
    }
    if request.quote_account.is_none() {
        return Err(EngineError::BadOrderParams("quote_account is missing"));
    }

    // enums. prost turns an unknown number into Unspecified aswell
    let side = request.side();
    let order_type = request.r#type();
    let time_in_force = request.time_in_force();
    if side == proto::Side::Unspecified {
        return Err(EngineError::BadOrderParams("side is not set"));
    }
    if order_type == OrderType::Unspecified {
        return Err(EngineError::BadOrderParams("type is not set"));
    }
    if time_in_force == TimeInForce::Unspecified {
        return Err(EngineError::BadOrderParams("time_in_force is not set"));
    }

    let is_limit = is_limit_type(order_type);
    let is_market = is_market_type(order_type);
    let is_stop = is_stop_type(order_type);

    // amount has to be in the unit the order needs
    let quantity = match (request.amount, wants_quote(order_type, side)) {
        (None, _) => return Err(EngineError::BadOrderParams("amount is not set")),
        (Some(Amount::BaseQuantity(quantity)), false) => quantity,
        (Some(Amount::QuoteQuantity(quantity)), true) => quantity,
        (Some(Amount::BaseQuantity(_)), true) => {
            return Err(EngineError::BadOrderParams(
                "market buys need quote_quantity, not base_quantity",
            ));
        }
        (Some(Amount::QuoteQuantity(_)), false) => {
            return Err(EngineError::BadOrderParams(
                "only market buys take quote_quantity, this order needs base_quantity",
            ));
        }
    };
    if u128::from(quantity) == 0 {
        return Err(EngineError::QuantityNotPositive);
    }

    // a price that is sent cant be zero
    if is_zero(request.limit_price)
        || is_zero(request.trigger_price)
        || is_zero(request.protection_price)
    {
        return Err(EngineError::PriceNotPositive);
    }

    // limit_price only on limit types and they must have it
    let has_limit_price = request.limit_price.is_some();
    if is_limit && !has_limit_price {
        return Err(EngineError::BadOrderParams(
            "LIMIT and STOP_LIMIT orders need a limit_price",
        ));
    }
    if !is_limit && has_limit_price {
        return Err(EngineError::BadOrderParams(
            "only LIMIT and STOP_LIMIT orders take a limit_price",
        ));
    }

    // trigger only on stop types and they must have it
    let has_trigger_price = request.trigger_price.is_some();
    let has_trigger_direction = request.trigger_direction() != TriggerDirection::Unspecified;
    if is_stop && (!has_trigger_price || !has_trigger_direction) {
        return Err(EngineError::BadOrderParams(
            "stop orders need a trigger_price and a trigger_direction",
        ));
    }
    if !is_stop && (has_trigger_price || has_trigger_direction) {
        return Err(EngineError::BadOrderParams(
            "only stop orders take a trigger_price or a trigger_direction",
        ));
    }

    // expire_time only on GTD and GTD must have it
    // TODO: check its in the future once GTD works (needs a clock)
    let is_gtd = time_in_force == TimeInForce::Gtd;
    let has_expire_time = request.expire_time.is_some();
    if is_gtd && !has_expire_time {
        return Err(EngineError::BadOrderParams(
            "GTD orders need an expire_time",
        ));
    }
    if !is_gtd && has_expire_time {
        return Err(EngineError::BadOrderParams(
            "only GTD orders take an expire_time",
        ));
    }

    // protection_price is optional and only for market types
    if !is_market && request.protection_price.is_some() {
        return Err(EngineError::BadOrderParams(
            "only market orders take a protection_price",
        ));
    }

    // its valid but we only run GTC limit orders for now
    if order_type != OrderType::Limit {
        return Err(EngineError::Unsupported("only LIMIT orders"));
    }
    if time_in_force != TimeInForce::Gtc {
        return Err(EngineError::Unsupported("only GTC limit orders"));
    }

    Ok(())
}

/// turns a checked request into a [`RestingOrder`]
///
/// only call it after [`check_place_order`] then none of these can fail
fn to_resting_order(request: PlaceOrderRequest) -> Result<RestingOrder> {
    let command_id = CommandId::new(required_id(request.command_id)?);
    let order_id = OrderId::new(required_id(request.order_id)?);
    let account_holder = UserId::new(required_id(request.account_holder)?);
    let side = side_from_proto(request.side())?;
    let price = Price::try_from(required(request.limit_price)?)?;
    let quantity = base_quantity_from_proto(request.amount)?;

    RestingOrder::new(order_id, account_holder, side, price, quantity, command_id)
}

/// turns a [`PlaceOrderRequest`] into a [`RestingOrder`] for the book
///
/// the whole request is checked first and only then its converted
pub fn parse_place_order(request: PlaceOrderRequest) -> Result<RestingOrder> {
    check_place_order(&request)?;
    to_resting_order(request)
}
