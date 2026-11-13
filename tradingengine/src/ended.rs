use std::collections::{HashMap, VecDeque};

use crate::ids::{OrderId, UserId};
use crate::order::OrderStatus;
use crate::resting_order::RestingOrder;

/// What a market keeps of an order after it has left the [`Book`](crate::book::Book).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndedOrder {
    /// The account holder who placed the order.
    pub owner: UserId,
    /// How the order ended: `Filled` or `Cancelled`.
    pub status: OrderStatus,
}

/// The orders that have left a market's book because they filled or were cancelled.
///
/// The book forgets an order once it ends, but the contract (`proto/how_to_use.md`,
/// "Validating a cancel or modify command") still needs it: a cancel or modify of an ended
/// order gets `ORDER_NOT_WORKING` with its final status instead of `UNKNOWN_ORDER`, and a new
/// order can't take its id.
///
/// Only the newest [`DEFAULT_CAPACITY`](Self::DEFAULT_CAPACITY) are kept. When one more ends,
/// the oldest is forgotten, and from then on it gets `UNKNOWN_ORDER` like an order that never
/// existed.
///
/// This is part of the market's state, like the book, so it must be saved and restored with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedOrders {
    by_id: HashMap<OrderId, EndedOrder>,
    /// The same ids in the order they ended, oldest first, so the oldest can be forgotten.
    oldest_first: VecDeque<OrderId>,
    capacity: usize,
}

impl EndedOrders {
    /// How many ended orders a market remembers. `proto/how_to_use.md` promises this number to
    /// the backend, so change it there too.
    pub const DEFAULT_CAPACITY: usize = 100_000;

    /// Remembers at most `capacity` ended orders.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            by_id: HashMap::new(),
            oldest_first: VecDeque::new(),
            capacity,
        }
    }

    /// Remembers an order that has just ended. If that makes one too many, the oldest is
    /// forgotten.
    pub fn record(&mut self, order: &RestingOrder) {
        debug_assert!(
            order.status().is_terminal(),
            "only an order that has ended is recorded"
        );
        self.insert(
            order.id(),
            EndedOrder {
                owner: order.user_id(),
                status: order.status(),
            },
        );
    }

    /// Remembers an ended order, like [`record`](Self::record). Saved state comes back this
    /// way, oldest first.
    pub fn insert(&mut self, id: OrderId, ended: EndedOrder) {
        // an id that is already here keeps its place in the line
        if self.by_id.insert(id, ended).is_none() {
            self.oldest_first.push_back(id);
        }
        while self.oldest_first.len() > self.capacity {
            if let Some(oldest) = self.oldest_first.pop_front() {
                self.by_id.remove(&oldest);
            }
        }
    }

    pub fn get(&self, id: OrderId) -> Option<&EndedOrder> {
        self.by_id.get(&id)
    }

    pub fn contains(&self, id: OrderId) -> bool {
        self.by_id.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The order that ended last, if any.
    pub fn newest(&self) -> Option<OrderId> {
        self.oldest_first.back().copied()
    }

    /// The ended orders, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = (OrderId, &EndedOrder)> {
        self.after(None)
    }

    /// The orders that ended after `id`, oldest first. If `id` is `None`, or is no longer
    /// remembered, that is every order here.
    pub fn after(&self, id: Option<OrderId>) -> impl Iterator<Item = (OrderId, &EndedOrder)> {
        let start = id
            .and_then(|id| self.oldest_first.iter().rposition(|&ended| ended == id))
            .map_or(0, |position| position + 1);
        self.oldest_first
            .iter()
            .skip(start)
            .filter_map(|&id| self.by_id.get(&id).map(|ended| (id, ended)))
    }
}

impl Default for EndedOrders {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}
