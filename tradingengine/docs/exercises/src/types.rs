use tradingengine::ids::{CommandId, OrderId, UserId};
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::{BaseQuantity, QuoteQuantity};

// a PlaceOrderRequest that passed validation. In slice 2 the only order the
// engine accepts is a limit order, GTC, with a base amount
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitGtc {
    pub command_id: CommandId,
    pub order_id: OrderId,
    pub symbol: String,
    pub account_holder: UserId,
    pub side: Side,
    pub price: Price,
    pub quantity: BaseQuantity,
}

// what the engine answers for an accepted place, before it becomes wire
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accepted {
    pub status: OrderStatus,
    pub filled_base: BaseQuantity,
    pub filled_quote: QuoteQuantity,
}
