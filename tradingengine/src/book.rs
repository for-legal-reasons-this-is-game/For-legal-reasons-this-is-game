use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::Side;
use crate::price::Price;
use crate::resting_order::RestingOrder;

type Level = VecDeque<RestingOrder>;
type BookSide = BTreeMap<Price, Level>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Book {
    bids: BookSide,
    asks: BookSide,
    orders: HashMap<OrderId, (Side, Price)>,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, order: RestingOrder) -> Result<()> {
        let id = order.id();
        let side = order.side();
        let price = order.price();

        let Entry::Vacant(slot) = self.orders.entry(id) else {
            return Err(EngineError::DuplicateOrderId);
        };
        slot.insert((side, price));

        self.side_mut(side)
            .entry(price)
            .or_default()
            .push_back(order);
        Ok(())
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

    pub fn amend(&mut self, id: OrderId, replacement: RestingOrder) -> Result<RestingOrder> {
        let replacement_id = replacement.id();
        if replacement_id != id && self.orders.contains_key(&replacement_id) {
            return Err(EngineError::DuplicateOrderId);
        }

        let replaced = self.remove(id)?;
        self.insert(replacement)?;
        Ok(replaced)
    }

    pub fn get(&self, id: OrderId) -> Option<&RestingOrder> {
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
