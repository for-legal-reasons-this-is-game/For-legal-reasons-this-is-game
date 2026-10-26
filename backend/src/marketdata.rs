// The order book, for display. Every pod keeps its own copy in memory, built from the engine's market
// data stream (a separate subscription than the event stream, and it needs no leader: the stream is
// read only and the engine allows many subscribers). Nothing here is stored, and nothing decides from it.
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

use num_traits::FromPrimitive;
use proto::{BookDelta, BookSnapshot, SubscribeMarketDataRequest, market_data_item::Body};
use tokio::sync::watch;
use tokio::time::{Duration, sleep};

use crate::domain::Side;
use crate::v1::AppState;

const RECONNECT_WAIT: Duration = Duration::from_secs(2);

// symbol of the market -> its book
pub type Books = Arc<RwLock<HashMap<String, Book>>>;

// price -> total resting base quantity at that price, both raw integers
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Book {
    bids: BTreeMap<u128, u128>,
    asks: BTreeMap<u128, u128>,
}

impl Book {
    pub fn from_snapshot(s: &BookSnapshot) -> Result<Self, String> {
        let level = |l: &proto::PriceLevel| match (l.price, l.quantity) {
            (Some(p), Some(q)) => Ok((u128::from(p), u128::from(q))),
            _ => Err("a price level without price or quantity".to_string()),
        };
        Ok(Book {
            bids: s.bids.iter().map(level).collect::<Result<_, _>>()?,
            asks: s.asks.iter().map(level).collect::<Result<_, _>>()?,
        })
    }

    // quantity 0 removes the level, anything else is the new total of the level
    pub fn apply_delta(&mut self, d: &BookDelta) -> Result<(), String> {
        let price = d.price.map(u128::from).ok_or("a delta without price")?;
        let quantity = d
            .quantity
            .map(u128::from)
            .ok_or("a delta without quantity")?;
        let side = match Side::from_i32(d.side) {
            Some(Side::Buy) => &mut self.bids,
            Some(Side::Sell) => &mut self.asks,
            None => return Err(format!("a delta with side {}", d.side)),
        };
        if quantity == 0 {
            side.remove(&price);
        } else {
            side.insert(price, quantity);
        }
        Ok(())
    }

    // highest price first
    pub fn bids(&self) -> Vec<(u128, u128)> {
        self.bids.iter().rev().map(|(p, q)| (*p, *q)).collect()
    }

    // lowest price first
    pub fn asks(&self) -> Vec<(u128, u128)> {
        self.asks.iter().map(|(p, q)| (*p, *q)).collect()
    }
}

fn clear(books: &Books) {
    books.write().unwrap_or_else(|e| e.into_inner()).clear();
}

pub async fn run(state: AppState, mut stop: watch::Receiver<bool>) {
    while !*stop.borrow() {
        let why = session(&state, &mut stop).await;
        // a book we can't keep up to date is worse than none
        clear(&state.books);
        let Some(why) = why else { return };
        eprintln!("MARKET DATA: reconnecting: {why}");
        tokio::select! {
            _ = sleep(RECONNECT_WAIT) => {}
            _ = stop.changed() => {}
        }
    }
}

// None = stopped, Some(reason) = the session ended and has to start again
async fn session(state: &AppState, stop: &mut watch::Receiver<bool>) -> Option<String> {
    let mut stream = match state
        .engine_client
        .subscribe_market_data(SubscribeMarketDataRequest {})
        .await
    {
        Ok(stream) => stream,
        Err(status) => return Some(format!("couldn't subscribe: {status}")),
    };

    let mut last: Option<(u64, u64)> = None; // (epoch, md_sequence) of the previous item
    loop {
        let item = tokio::select! {
            _ = stop.changed() => return None,
            message = stream.message() => match message {
                Ok(Some(item)) => item,
                Ok(None) => return Some("the engine closed the stream".into()),
                Err(status) => return Some(format!("stream error: {status}")),
            },
        };

        // a gap (or a new engine) means the book can't be trusted: start over for a new snapshot
        if let Some((epoch, sequence)) = last
            && (item.engine_epoch != epoch || item.md_sequence != sequence + 1)
        {
            return Some(format!(
                "gap in the market data: after {sequence} came {}",
                item.md_sequence
            ));
        }
        last = Some((item.engine_epoch, item.md_sequence));

        let mut books = state.books.write().unwrap_or_else(|e| e.into_inner());
        match item.body {
            Some(Body::BookSnapshot(snapshot)) => match Book::from_snapshot(&snapshot) {
                Ok(book) => {
                    books.insert(item.market_symbol, book);
                }
                Err(why) => return Some(format!("bad snapshot of {}: {why}", item.market_symbol)),
            },
            Some(Body::BookDelta(delta)) => {
                // a market's snapshot always comes before its deltas
                let Some(book) = books.get_mut(&item.market_symbol) else {
                    return Some(format!(
                        "delta of {} before its snapshot",
                        item.market_symbol
                    ));
                };
                if let Err(why) = book.apply_delta(&delta) {
                    return Some(format!("bad delta of {}: {why}", item.market_symbol));
                }
            }
            None => eprintln!("MARKET DATA: item of an unknown kind, skipped"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(price: u128, quantity: u128) -> proto::PriceLevel {
        proto::PriceLevel {
            price: Some(price.into()),
            quantity: Some(quantity.into()),
        }
    }

    fn delta(side: Side, price: u128, quantity: u128) -> BookDelta {
        BookDelta {
            side: side as i32,
            price: Some(price.into()),
            quantity: Some(quantity.into()),
        }
    }

    #[test]
    fn a_snapshot_is_sorted_best_first() {
        let book = Book::from_snapshot(&BookSnapshot {
            bids: vec![level(99, 1), level(100, 2)],
            asks: vec![level(102, 3), level(101, 4)],
        })
        .unwrap();
        assert_eq!(book.bids(), vec![(100, 2), (99, 1)]);
        assert_eq!(book.asks(), vec![(101, 4), (102, 3)]);
    }

    #[test]
    fn deltas_set_and_remove_levels() {
        let mut book = Book::default();
        book.apply_delta(&delta(Side::Buy, 100, 5)).unwrap();
        book.apply_delta(&delta(Side::Buy, 100, 7)).unwrap(); // the new total, not an addition
        book.apply_delta(&delta(Side::Sell, 101, 1)).unwrap();
        assert_eq!(book.bids(), vec![(100, 7)]);
        book.apply_delta(&delta(Side::Buy, 100, 0)).unwrap();
        assert!(book.bids().is_empty());
        assert_eq!(book.asks(), vec![(101, 1)]);
    }

    #[test]
    fn incomplete_messages_are_errors() {
        let mut book = Book::default();
        assert!(
            book.apply_delta(&BookDelta {
                side: 1,
                price: None,
                quantity: Some(1u128.into())
            })
            .is_err()
        );
        assert!(book.apply_delta(&delta(Side::Buy, 1, 1)).is_ok());
        assert!(
            book.apply_delta(&BookDelta {
                side: 0,
                price: Some(1u128.into()),
                quantity: Some(1u128.into())
            })
            .is_err()
        );
        assert!(
            Book::from_snapshot(&BookSnapshot {
                bids: vec![proto::PriceLevel::default()],
                asks: vec![]
            })
            .is_err()
        );
    }
}
