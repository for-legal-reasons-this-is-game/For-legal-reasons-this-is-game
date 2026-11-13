use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::Side;
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;
use crate::resting_order::RestingOrder;

type Level = VecDeque<RestingOrder>; // all the orders at the same price 
type BookSide = BTreeMap<Price, Level>; // a map of one side of the book (sell/by) price is needed for faster access

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub maker: OrderId,
    pub taker: OrderId,
    pub price: Price,
    pub quantity: BaseQuantity,
    /// What the fill was worth in the quote asset, rounded down like its `TradeSettlement`.
    pub quote: QuoteQuantity,
}

/// The result returned after an order is matched against the [`Book`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Execution {
    fills: Vec<Fill>, // all the resting orders that were filled because of the new order
    resting: Option<OrderId>, // the remainder of the new order NONE if it was filled
    ended: Vec<RestingOrder>, // the orders that filled completely and left the book
}

impl Execution {
    pub fn fills(&self) -> &[Fill] {
        &self.fills
    }

    pub const fn resting_order_id(&self) -> Option<OrderId> {
        self.resting
    }

    /// The orders that filled completely and left the book, in the order they ended: the
    /// makers first, then the incoming order if nothing of it was left to rest.
    ///
    /// The book forgets them, so the caller has to remember them, in
    /// [`EndedOrders`](crate::ended::EndedOrders).
    pub fn ended(&self) -> &[RestingOrder] {
        &self.ended
    }

    pub fn total_filled_quantity(&self) -> Result<BaseQuantity> {
        self.fills
            .iter()
            .try_fold(BaseQuantity::ZERO, |total, fill| {
                total.checked_add(fill.quantity)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    bids: BookSide, // wants to buy
    asks: BookSide, // wants to sell
    all_orders: HashMap<OrderId, (Side, Price)>,
    // orders are a general index of each order id to prevent double insertion and
    // faster search instead of walking down a whole bookside
    scale: QuoteScale, // the market's scale, to work out what each fill is worth in quote
}

impl Book {
    /// An empty book for a market with this [`QuoteScale`].
    pub fn new(scale: QuoteScale) -> Self {
        Self {
            bids: BookSide::new(),
            asks: BookSide::new(),
            all_orders: HashMap::new(),
            scale,
        }
    }

    pub fn scale(&self) -> QuoteScale {
        self.scale
    }

    /// Puts a new incoming order into the book, unless it can be matched against a
    /// price on the opposite side of the book.
    ///
    /// The orders that fill completely leave the book and come back in
    /// [`Execution::ended`].
    pub fn insert(&mut self, mut order: RestingOrder) -> Result<Execution> {
        let id = order.id();
        if self.all_orders.contains_key(&id) {
            return Err(EngineError::DuplicateOrderId);
        }

        let (fills, mut ended) = self.match_incoming(&mut order)?;

        let resting = if order.qty_remaining().is_zero() {
            // filled completely on arrival, so it ends without ever resting
            ended.push(order);
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

        Ok(Execution {
            fills,
            resting,
            ended,
        })
    }

    /// Tries to match the incoming order against the other side of the book.
    ///
    /// Returns the [`Fill`]s it matched, and the makers that filled completely and left the
    /// book.
    fn match_incoming(
        &mut self,
        incoming: &mut RestingOrder,
    ) -> Result<(Vec<Fill>, Vec<RestingOrder>)> {
        let taker_side = incoming.side();
        let limit = incoming.price();
        let scale = self.scale;

        let opposite_side = match taker_side {
            Side::Buy => &mut self.asks,
            Side::Sell => &mut self.bids,
        };
        let orders = &mut self.all_orders;

        let mut fills = Vec::new();
        let mut ended = Vec::new();

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

                // both sides trade at the maker's price, so the fill is worth the same to each
                let quote = scale.quote_for(price, quantity)?;
                maker.fill(quantity, quote)?;
                let exhausted = maker.qty_remaining().is_zero();
                incoming.fill(quantity, quote)?;

                fills.push(Fill {
                    maker: maker_id,
                    taker: incoming.id(),
                    price,
                    quantity,
                    quote,
                });

                if exhausted {
                    if let Some(maker) = level.pop_front() {
                        ended.push(maker);
                    }
                    orders.remove(&maker_id);
                }
            }

            if level.is_empty() {
                opposite_side.remove(&price);
            }
        }

        Ok((fills, ended))
    }

    /// Takes an order out of the book and returns it, without changing its status.
    ///
    /// The order is found first and only then removed, so an id the book doesn't hold leaves
    /// the book as it was.
    pub fn remove(&mut self, id: OrderId) -> Result<RestingOrder> {
        let &(side, price) = self.all_orders.get(&id).ok_or(EngineError::OrderNotFound)?;
        let levels = self.side_mut(side);
        let level = levels.get_mut(&price).ok_or(EngineError::OrderNotFound)?;
        let position = level
            .iter()
            .position(|resting| resting.id() == id)
            .ok_or(EngineError::OrderNotFound)?;
        let order = level.remove(position).ok_or(EngineError::OrderNotFound)?;

        if level.is_empty() {
            levels.remove(&price);
        }
        self.all_orders.remove(&id);
        Ok(order)
    }

    /// Changes a resting order's quantity and/or price. `None` leaves that value as it was.
    ///
    /// The contract (`proto/how_to_use.md`) decides where the order ends up in the queue:
    /// - Only a lower (or the same) quantity at the same price keeps its place. The order
    ///   asks for less than it queued for, so nobody behind it loses out.
    /// - A higher quantity or a new price goes to the back of the line. A new price can
    ///   also cross the book and match right away, like a new order.
    ///
    /// A rejected modify leaves the book exactly as it was.
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
                resting: Some(id),
                ..Execution::default()
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
