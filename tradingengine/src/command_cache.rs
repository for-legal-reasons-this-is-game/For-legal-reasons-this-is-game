use std::collections::{HashMap, VecDeque};

use proto::{CancelOrderRequest, CancelOrderResponse, PlaceOrderRequest, PlaceOrderResponse};
use serde::{Deserialize, Serialize};

use crate::{
    error::{EngineError, Result},
    ids::CommandId,
    proto_bytes,
};

/// This will have to change accordingly with the period the backend takes for retries.
/// And imo shouldn't be hardcoded at all
pub const DEFAULT_CACHE_CAPACITY: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // this is a box because clippy screams.
    Place(#[serde(with = "proto_bytes")] Box<PlaceOrderRequest>),
    Cancel(#[serde(with = "proto_bytes")] CancelOrderRequest),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Answer {
    Place(#[serde(with = "proto_bytes")] PlaceOrderResponse),
    Cancel(#[serde(with = "proto_bytes")] CancelOrderResponse),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandCache {
    by_id: HashMap<CommandId, (Command, Answer)>,
    oldest_first: VecDeque<CommandId>,
    capacity: usize,
}

impl Default for CommandCache {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CACHE_CAPACITY,
            by_id: HashMap::with_capacity(DEFAULT_CACHE_CAPACITY),
            oldest_first: VecDeque::with_capacity(DEFAULT_CACHE_CAPACITY),
        }
    }
}

impl CommandCache {
    /// Returns a [`CommandCache`] with [`CommandCache::capacity`] set to `n`
    ///   and both [`CommandCache::by_id`] and [`CommandCache::oldest_first`]
    ///   called with `n` capacity
    /// `n == 0` is treated as [`DEFAULT_CACHE_CAPACITY`]
    pub fn with_capacity(mut n: usize) -> Self {
        if n == 0 {
            n = DEFAULT_CACHE_CAPACITY;
        }
        CommandCache {
            capacity: n,
            by_id: HashMap::with_capacity(n),
            oldest_first: VecDeque::with_capacity(n),
        }
    }

    /// Returns the cache, and the ids of the oldest entries that didn't fit in `n`. Those
    /// are only there when `n` is lower than when the entries were saved.
    pub fn load_cache(
        n: usize,
        oldest_first: Vec<(CommandId, Command, Answer)>,
    ) -> (Self, Vec<CommandId>) {
        let mut command_cache = CommandCache::with_capacity(n);
        let mut removed = Vec::new();
        for (command_id, command, answer) in oldest_first {
            removed.extend(command_cache.add_entry(command_id, command, answer));
        }
        (command_cache, removed)
    }

    pub fn get_cache(&self) -> Vec<(CommandId, Command, Answer)> {
        let mut res = Vec::new();
        for id in &self.oldest_first {
            let (command, answer) = self
                .by_id
                .get(id)
                .expect("error while saving `CommandCache`");
            res.push((*id, command.clone(), answer.clone()));
        }
        res
    }

    /// - Inserts the key `command_id` and the value (`request`, `response`)
    ///   to the cache hashmap [`CommandCache::by_id`].
    /// - Pushes the `command_id` to the VecDeque [`CommandCache::oldest_first`].
    /// - Returns `None` if nothing was removed from the cache, otherwise returns the removed [`CommandId`]
    pub fn add_entry(
        &mut self,
        command_id: CommandId,
        request: Command,
        response: Answer,
    ) -> Option<CommandId> {
        let mut removed: Option<CommandId> = None;
        if self.by_id.len() == self.capacity {
            removed = Some(
                self.oldest_first
                    .pop_front()
                    .expect("error on adding entry to cache: `CommandCache::add_entry`"),
            );
            self.by_id
                .remove(&removed.unwrap())
                .expect("error on adding entry to cache: `CommandCache::add_entry`");
        }
        if self.by_id.insert(command_id, (request, response)).is_some() {
            panic!("error in `CommandCache`, found identical `command_id` on inserting an entry")
        }
        self.oldest_first.push_back(command_id);
        removed
    }

    /// - Returns `Ok(Some(response))` for a found `command_id` that has identical
    ///   hash (`cache_request` == `request`).
    /// - Returns [`EngineError::DuplicateCommand`] for a found `command_id` that has different hash
    /// - Returns [`Ok(None)`] if no `command_id` found in cache.
    ///
    /// For Redis I assume we will search each command separately (ask redis for a place_order request).
    pub fn search(&self, command_id: CommandId, request: &Command) -> Result<Option<&Answer>> {
        match self.by_id.get(&command_id) {
            None => Ok(None),
            Some((cached_request, response)) if cached_request == request => Ok(Some(response)),
            Some(_) => Err(EngineError::DuplicateCommand),
        }
    }
}
