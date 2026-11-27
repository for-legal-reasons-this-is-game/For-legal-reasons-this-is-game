use serde::{Deserialize, Serialize};

use crate::error::{EngineError, Result};
use crate::ids::{AccountId, CommandId, OrderId, UserId};
use crate::order::{OrderStatus, Side};
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};

/// What a `TradeSettlement` needs about one order: the accounts its money moves between.
/// The engine doesn't use them; it stores them and sends them back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementInfo {
    pub base_account: AccountId,
    pub quote_account: AccountId,
}

/// An order sitting in the [`Book`](crate::book::Book).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestingOrder {
    id: OrderId,
    user_id: UserId,
    side: Side,
    price: Price,
    qty_total: BaseQuantity,
    qty_remaining: BaseQuantity,
    filled_quote: QuoteQuantity,
    status: OrderStatus,
    command_id: CommandId,
    settlement: SettlementInfo,
}

impl RestingOrder {
    pub fn new(
        id: OrderId,
        user_id: UserId,
        side: Side,
        price: Price,
        qty_total: BaseQuantity,
        command_id: CommandId,
        settlement: SettlementInfo,
    ) -> Result<Self> {
        let qty_total = qty_total.require_positive()?;

        Ok(Self {
            id,
            user_id,
            side,
            price,
            qty_total,
            qty_remaining: qty_total,
            filled_quote: QuoteQuantity::ZERO,
            status: OrderStatus::Open,
            command_id,
            settlement,
        })
    }

    pub fn fill(&mut self, quantity: BaseQuantity, quote: QuoteQuantity) -> Result<()> {
        let quantity = quantity.require_positive()?;
        if quantity > self.qty_remaining {
            return Err(EngineError::FillExceedsRemaining);
        }

        let qty_remaining = self.qty_remaining.checked_sub(quantity)?;
        let filled_quote = self.filled_quote.checked_add(quote)?;
        let status = if qty_remaining.is_zero() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        self.qty_remaining = qty_remaining;
        self.filled_quote = filled_quote;
        self.status = status;
        Ok(())
    }

    /// Changes the order's total quantity and/or price. `None` leaves that value as it was.
    ///
    /// The new total must be above what is already filled (`BAD_QUANTITY` in the
    /// contract), and the remaining quantity becomes the new total minus the filled part.
    /// On an error, nothing changes.
    ///
    /// Where the order sits in the queue is up to [`Book::modify`](crate::book::Book::modify).
    pub fn modify(
        &mut self,
        new_quantity: Option<BaseQuantity>,
        new_price: Option<Price>,
    ) -> Result<()> {
        if self.status.is_terminal() {
            return Err(EngineError::OrderNotLive);
        }

        let filled = self.qty_total.checked_sub(self.qty_remaining)?;
        let new_quantity = new_quantity.unwrap_or(self.qty_total);
        if new_quantity <= filled {
            return Err(EngineError::QuantityNotAboveFilled);
        }

        self.qty_remaining = new_quantity.checked_sub(filled)?;
        self.qty_total = new_quantity;
        if let Some(price) = new_price {
            self.price = price;
        }
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<()> {
        if self.status.is_terminal() {
            return Err(EngineError::OrderNotLive);
        }

        self.status = OrderStatus::Cancelled;
        Ok(())
    }

    pub const fn id(&self) -> OrderId {
        self.id
    }

    pub const fn user_id(&self) -> UserId {
        self.user_id
    }

    pub const fn side(&self) -> Side {
        self.side
    }

    pub const fn price(&self) -> Price {
        self.price
    }

    pub const fn qty_total(&self) -> BaseQuantity {
        self.qty_total
    }

    pub const fn qty_remaining(&self) -> BaseQuantity {
        self.qty_remaining
    }

    pub const fn filled_quote(&self) -> QuoteQuantity {
        self.filled_quote
    }

    pub const fn settlement(&self) -> SettlementInfo {
        self.settlement
    }

    pub const fn status(&self) -> OrderStatus {
        self.status
    }

    /// Returned by value, now that [`CommandId`] is a plain number like [`OrderId`].
    pub const fn command_id(&self) -> CommandId {
        self.command_id
    }
}
