use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::Side;
use crate::price::Price;
use crate::quantity::Quantity;
use crate::resting_order::RestingOrder;

type Level = VecDeque<RestingOrder>;
type BookSide = BTreeMap<Price, Level>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    maker: OrderId,
    taker: OrderId,
    price: Price,
    quantity: Quantity,
}

impl Fill {
    pub const fn maker(&self) -> OrderId {
        self.maker
    }

    pub const fn taker(&self) -> OrderId {
        self.taker
    }

    pub const fn price(&self) -> Price {
        self.price
    }

    pub const fn quantity(&self) -> Quantity {
        self.quantity
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Execution {
    fills: Vec<Fill>,
    resting: Option<OrderId>,
}

impl Execution {
    pub fn fills(&self) -> &[Fill] {
        &self.fills
    }

    pub const fn resting(&self) -> Option<OrderId> {
        self.resting
    }

    pub fn filled(&self) -> Result<Quantity> {
        self.fills.iter().try_fold(Quantity::ZERO, |total, fill| {
            total.checked_add(fill.quantity)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Book {
    bids: BookSide, // wants to buy
    asks: BookSide, // wants to sell
    orders: HashMap<OrderId, (Side, Price)>,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, mut order: RestingOrder) -> Result<Execution> {
        let id = order.id();
        if self.orders.contains_key(&id) {
            return Err(EngineError::DuplicateOrderId);
        }

        let fills = self.match_incoming(&mut order)?;

        let resting = if order.qty_remaining().is_zero() {
            None
        } else {
            let side = order.side();
            let price = order.price();
            self.orders.insert(id, (side, price));
            self.side_mut(side)
                .entry(price)
                .or_default()
                .push_back(order);
            Some(id)
        };

        Ok(Execution { fills, resting })
    }

    fn match_incoming(&mut self, incoming: &mut RestingOrder) -> Result<Vec<Fill>> {
        let taker_side = incoming.side();
        let limit = incoming.price();

        let levels = match taker_side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };
        let orders = &mut self.orders;

        let mut fills = Vec::new();

        while !incoming.qty_remaining().is_zero() {
            let best = match taker_side {
                Side::Buy => levels.keys().next().copied(),
                Side::Sell => levels.keys().next_back().copied(),
            };
            let Some(price) = best else {
                break;
            };

            let crosses = match taker_side {
                Side::Buy => price <= limit,
                Side::Sell => price >= limit,
            };
            if !crosses {
                break;
            }

            let Some(level) = levels.get_mut(&price) else {
                break;
            };

            while !incoming.qty_remaining().is_zero() {
                let Some(maker) = level.front_mut() else {
                    break;
                };

                let maker_id = maker.id();
                let quantity = incoming.qty_remaining().min(maker.qty_remaining());

                maker.fill(quantity)?;
                let exhausted = maker.qty_remaining().is_zero();
                incoming.fill(quantity)?;

                fills.push(Fill {
                    maker: maker_id,
                    taker: incoming.id(),
                    price,
                    quantity,
                });

                if exhausted {
                    level.pop_front();
                    orders.remove(&maker_id);
                }
            }

            if level.is_empty() {
                levels.remove(&price);
            }
        }

        Ok(fills)
    }

    pub fn remove(&mut self, id: OrderId) -> Result<RestingOrder> {
        let (side, price) = self.orders.remove(&id).ok_or(EngineError::OrderNotFound)?;
        let levels = self.side_mut(side);

        let Some(level) = levels.get_mut(&price) else {
            return Err(EngineError::OrderNotFound);
        };
        let Some(position) = level.iter().position(|resting| resting.id() == id) else {
            return Err(EngineError::OrderNotFound);
        };
        let Some(order) = level.remove(position) else {
            return Err(EngineError::OrderNotFound);
        };

        if level.is_empty() {
            levels.remove(&price);
        }
        Ok(order)
    }

    pub fn amend(
        &mut self,
        id: OrderId,
        replacement: RestingOrder,
    ) -> Result<(RestingOrder, Execution)> {
        let replacement_id = replacement.id();
        if replacement_id != id && self.orders.contains_key(&replacement_id) {
            return Err(EngineError::DuplicateOrderId);
        }

        let replaced = self.remove(id)?;
        let execution = self.insert(replacement)?;
        Ok((replaced, execution))
    }

    pub fn get_by_id(&self, id: OrderId) -> Option<&RestingOrder> {
        let &(side, price) = self.orders.get(&id)?;
        self.side(side)
            .get(&price)?
            .iter()
            .find(|resting| resting.id() == id)
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn bids(&self) -> impl Iterator<Item = &RestingOrder> {
        self.bids.values().rev().flat_map(Level::iter)
    }

    pub fn asks(&self) -> impl Iterator<Item = &RestingOrder> {
        self.asks.values().flat_map(Level::iter)
    }

    pub fn len(&self) -> usize {
        self.orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    fn side(&self, side: Side) -> &BookSide {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut BookSide {
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }
}
