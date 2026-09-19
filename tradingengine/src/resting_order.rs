use crate::error::{EngineError, Result};
use crate::ids::{IdempotencyKey, OrderId, SeqNo, UserId};
use crate::order::{OrderStatus, Side};
use crate::price::Price;
use crate::quantity::Quantity;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestingOrder {
    id: OrderId,
    user_id: UserId,
    side: Side,
    price: Price,
    qty_original: Quantity,
    qty_remaining: Quantity,
    status: OrderStatus,
    seq_no: SeqNo,
    idempotency_key: IdempotencyKey,
}

impl RestingOrder {
    pub fn new(
        id: OrderId,
        user_id: UserId,
        side: Side,
        price: Price,
        qty_original: Quantity,
        seq_no: SeqNo,
        idempotency_key: IdempotencyKey,
    ) -> Result<Self> {
        let qty_original = qty_original.require_positive()?;

        Ok(Self {
            id,
            user_id,
            side,
            price,
            qty_original,
            qty_remaining: qty_original,
            status: OrderStatus::Open,
            seq_no,
            idempotency_key,
        })
    }

    pub fn fill(&mut self, quantity: Quantity) -> Result<()> {
        let quantity = quantity.require_positive()?;
        if quantity > self.qty_remaining {
            return Err(EngineError::FillExceedsRemaining);
        }

        let qty_remaining = self.qty_remaining.checked_sub(quantity)?;
        let status = if qty_remaining.is_zero() {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        self.qty_remaining = qty_remaining;
        self.status = status;
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

    pub const fn qty_original(&self) -> Quantity {
        self.qty_original
    }

    pub const fn qty_remaining(&self) -> Quantity {
        self.qty_remaining
    }

    pub const fn status(&self) -> OrderStatus {
        self.status
    }

    pub const fn seq_no(&self) -> SeqNo {
        self.seq_no
    }

    pub fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }
}
