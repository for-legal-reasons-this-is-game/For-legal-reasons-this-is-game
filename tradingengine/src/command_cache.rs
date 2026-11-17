use std::collections::{HashMap, VecDeque};

use proto::{CancelOrderRequest, CancelOrderResponse, Id, PlaceOrderRequest, PlaceOrderResponse};

use crate::error::{EngineError, Result};

/// This will have to change accordingly with the period the backend takes for retries.
/// And imo shouldn't be hardcoded at all
const DEFAULT_CAPACITY: usize = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // this is a box because clippy screams, change this whole enum better.
    Place(Box<PlaceOrderRequest>),
    Cancel(CancelOrderRequest),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Place(PlaceOrderResponse),
    Cancel(CancelOrderResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandCache {
    by_id: HashMap<Id, (Command, Answer)>,
    oldest_first: VecDeque<Id>,
    capacity: usize,
}

impl Default for CommandCache {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            by_id: HashMap::with_capacity(DEFAULT_CAPACITY),
            oldest_first: VecDeque::with_capacity(DEFAULT_CAPACITY),
        }
    }
}

impl CommandCache {
    /// Returns a [`CommandCache`] with [`CommandCache::capacity`] set to `n`
    ///   and both [`CommandCache::by_id`] and [`CommandCache::oldest_first`]
    ///   called with `n` capacity
    /// `n == 0` is treated as [`DEFAULT_CAPACITY`]
    pub fn with_capacity(mut n: usize) -> Self {
        if n == 0 {
            n = DEFAULT_CAPACITY;
        }
        CommandCache {
            capacity: n,
            by_id: HashMap::with_capacity(n),
            oldest_first: VecDeque::with_capacity(n),
        }
    }

    /// - Inserts the key `command_id` and the value (`request`, `response`)
    ///   to the cache hashmap [`CommandCache::by_id`].
    /// - Pushes the `command_id` to the VecDeque [`CommandCache::oldest_first`].
    pub fn add_entry(&mut self, command_id: Id, request: Command, response: Answer) {
        if self.by_id.len() == self.capacity {
            let removed = self
                .oldest_first
                .pop_front()
                .expect("major bug that will never happen just happened");
            self.by_id
                .remove(&removed)
                .expect("major bug that will never happen just happened");
        }
        if self.by_id.insert(command_id, (request, response)).is_some() {
            panic!(
                "major bug in Engine::command_cache, found identical command_id on inserting an entry"
            )
        }
        self.oldest_first.push_back(command_id);
    }

    /// - Returns `Ok(Some(response))` for a found `command_id` that has identical
    ///   hash (`cache_request` == `request`).
    /// - Returns [`EngineError::DuplicateCommand`] for a found `command_id` that has different hash
    /// - Returns [`Ok(None)`] if no `command_id` found in cache.
    ///
    /// For Redis I assume we will search each command separately (ask redis for a place_order request).
    pub fn search(&self, command_id: Id, request: &Command) -> Result<Option<&Answer>> {
        match self.by_id.get(&command_id) {
            None => Ok(None),
            Some((cached_request, response)) if cached_request == request => Ok(Some(response)),
            Some(_) => Err(EngineError::DuplicateCommand),
        }
    }
}
