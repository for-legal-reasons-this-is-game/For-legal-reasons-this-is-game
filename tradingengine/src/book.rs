use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::Side;
use crate::price::Price;
use crate::quantity::BaseQuantity;
use crate::resting_order::RestingOrder;

type Level = VecDeque<RestingOrder>; // all the orders at the same price 
type BookSide = BTreeMap<Price, Level>; // a map of one side of the book (sell/by) price is needed for faster access

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub maker: OrderId,
    pub taker: OrderId,
    pub price: Price,
    pub quantity: BaseQuantity,
}

// after a matching happens a an execution struct is returned
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Execution {
    fills: Vec<Fill>, // all the resting orders that were filled because of the new order
    resting: Option<OrderId>, // the remainder of the new order NONE if it was filled
}

impl Execution {
    pub fn fills(&self) -> &[Fill] {
        &self.fills
    }

    pub const fn resting_order_id(&self) -> Option<OrderId> {
        self.resting
    }

    pub fn total_filled_quantity(&self) -> Result<BaseQuantity> {
        self.fills
            .iter()
            .try_fold(BaseQuantity::ZERO, |total, fill| {
                total.checked_add(fill.quantity)
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Book {
    bids: BookSide, // wants to buy
    asks: BookSide, // wants to sell
    all_orders: HashMap<OrderId, (Side, Price)>,
    // orders are a general index of each order id to prevent double insertion and
    // faster search instead of walking down a whole bookside
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    // insert tries to put the new incomming order into the book unless the order
    // can be matched to the a price on the oposite side of the book
    pub fn insert(&mut self, mut order: RestingOrder) -> Result<Execution> {
        let id = order.id();
        if self.all_orders.contains_key(&id) {
            return Err(EngineError::DuplicateOrderId);
        }

        let fills = self.match_incoming(&mut order)?;

        let resting = if order.qty_remaining().is_zero() {
            None
        } else {
            let side = order.side();
            let price = order.price();
            self.all_orders.insert(id, (side, price));
            self.side_mut(side)
                .entry(price)
                .or_default()
                .push_back(order);
            Some(id)
        };

        Ok(Execution { fills, resting })
    }

    // tries to match the incoming order to a the other side
    // returns fills(matched)
    fn match_incoming(&mut self, incoming: &mut RestingOrder) -> Result<Vec<Fill>> {
        let taker_side = incoming.side();
        let limit = incoming.price();

        let opposite_side = match taker_side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };
        let orders = &mut self.all_orders;

        let mut fills = Vec::new();

        while !incoming.qty_remaining().is_zero() {
            let best = match taker_side {
                Side::Buy => opposite_side.keys().next().copied(),
                Side::Sell => opposite_side.keys().next_back().copied(),
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

            let Some(level) = opposite_side.get_mut(&price) else {
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
                opposite_side.remove(&price);
            }
        }

        Ok(fills)
    }

    pub fn remove(&mut self, id: OrderId) -> Result<RestingOrder> {
        let (side, price) = self
            .all_orders
            .remove(&id)
            .ok_or(EngineError::OrderNotFound)?;
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

    // changes a resting order's quantity and/or price, None leaves it as it was.
    // the contract (proto/how_to_use.md) decides where it ends up in the queue:
    // - only a lower (or the same) quantity at the same price keeps its place,
    //   it asks for less than it queued for so nobody behind it loses out
    // - a higher quantity or a new price goes to the back of the line, and a
    //   new price can cross the book and match right away like a new order
    // a rejected modify leaves the book exactly as it was
    pub fn modify(
        &mut self,
        id: OrderId,
        new_quantity: Option<BaseQuantity>,
        new_price: Option<Price>,
    ) -> Result<Execution> {
        let &(side, price) = self.all_orders.get(&id).ok_or(EngineError::OrderNotFound)?;
        let order = self
            .side_mut(side)
            .get_mut(&price)
            .and_then(|level| level.iter_mut().find(|resting| resting.id() == id))
            .ok_or(EngineError::OrderNotFound)?;

        let same_price = new_price.is_none_or(|new| new == price);
        let not_bigger = new_quantity.is_none_or(|new| new <= order.qty_total());

        if same_price && not_bigger {
            // changed in place, so it keeps its position in the level
            order.modify(new_quantity, None)?;
            return Ok(Execution {
                fills: Vec::new(),
                resting: Some(id),
            });
        }

        // check the change on a copy first so a rejected modify has not
        // already taken the order out of the book
        let mut changed = order.clone();
        changed.modify(new_quantity, new_price)?;

        self.remove(id)?;
        self.insert(changed)
    }

    pub fn get_by_id(&self, id: OrderId) -> Option<&RestingOrder> {
        let &(side, price) = self.all_orders.get(&id)?;
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
        self.all_orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all_orders.is_empty()
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
