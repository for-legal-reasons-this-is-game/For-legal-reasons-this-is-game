use std::time::Duration;

use prost::Message;
use proto::{EngineCommand, EventItem, engine_command::Body};
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

use crate::{engine::Engine, engine::State, event_sender::NewEvents, redis_store::RedisStore};

const POP_TIMEOUT: f64 = 5.0;
/// This time-to-live is how long an answer (that reached redis, but
/// the router hasn't read yet) stays in redis before it gets deleted,
/// and in that case it's a timeout in backend so it retries. the
/// router will need to have a timeout that is smaller than this (will
/// likely be 5-10 secs), because if it didn't, the answer could be
/// deleted before the router reads it.
const ANSWER_TTL_SECONDS: i64 = 60;

pub struct RouterConsumer {
    connection: ConnectionManager,
    commands_key: String,
    state: State,
    /// where the events of each command go after the state is saved. `None` drops them
    new_events: Option<NewEvents>,
}

impl RouterConsumer {
    /// Connects to `Redis` at `url`.
    /// Returns (`ConnectionManager`, `commands_key` made from `instance`).
    async fn connect(url: &str, instance: &str) -> redis::RedisResult<(ConnectionManager, String)> {
        let client = redis::Client::open(url)?;
        let config = ConnectionManagerConfig::new()
            .set_response_timeout(Some(Duration::from_secs_f64(POP_TIMEOUT + 5.0)));
        let mut connection = ConnectionManager::new_with_config(client, config).await?;

        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok((connection, format!("task:pending:{instance}")))
    }

    pub async fn new(
        engine: Engine,
        store: Option<RedisStore>,
        router_url: &str,
        instance: &str,
    ) -> redis::RedisResult<Self> {
        let (connection, commands_key) = RouterConsumer::connect(router_url, instance).await?;
        Ok(RouterConsumer {
            connection,
            commands_key,
            state: State { engine, store },
            new_events: None,
        })
    }

    /// sends the events of every command to `new_events`, see
    /// [`EventSender`](crate::event_sender::EventSender)
    pub fn send_events_to(&mut self, new_events: NewEvents) {
        self.new_events = Some(new_events);
    }

    /// Pops next encoded [`EngineCommand`] from `redis`.
    async fn pop(&mut self) -> redis::RedisResult<Option<Vec<u8>>> {
        let popped: Option<(String, Vec<u8>)> = self
            .connection
            .blpop(&self.commands_key, POP_TIMEOUT)
            .await?;
        Ok(popped.map(|(_, bytes)| bytes))
    }

    /// Decodes command's bytes.
    /// Returns decoded [`EngineCommand`] object.
    fn decode(bytes: Vec<u8>) -> Option<EngineCommand> {
        match EngineCommand::decode(bytes.as_slice()) {
            Ok(command) => Some(command),
            Err(error) => {
                eprintln!(
                    "couldn't decode `EngineCommand`'s bytes from the router, skipping it. error: {error}"
                );
                None
            }
        }
    }

    /// Runs the command.
    /// Returns the encoded answer and the events the command made
    fn run(&mut self, body: Option<Body>) -> Option<(Vec<u8>, Vec<EventItem>)> {
        let Some(body) = body else {
            eprintln!("error in router consumer: empty body, continuing...");
            return None;
        };
        let engine = &mut self.state.engine;
        let answer = match body {
            Body::SetMarket(request) => (engine.set_market(request).encode_to_vec(), Vec::new()),
            Body::PlaceOrder(request) => {
                let (response, events) = engine.place_order_with_events(request);
                (response.encode_to_vec(), events)
            }
            Body::CancelOrder(request) => {
                let (response, events) = engine.cancel_order_with_events(request);
                (response.encode_to_vec(), events)
            }
        };
        Some(answer)
    }

    /// Pushes response (`answer`) back to `redis` with a `ANSWER_TTL_SECONDS` timeout
    async fn answer(&mut self, reply_to: String, answer: Vec<u8>) -> redis::RedisResult<()> {
        redis::pipe()
            .atomic()
            .rpush(&reply_to, answer)
            .ignore()
            .expire(reply_to, ANSWER_TTL_SECONDS)
            .ignore()
            .query_async(&mut self.connection)
            .await
    }

    pub async fn handle_next(&mut self) -> redis::RedisResult<()> {
        let Some(bytes) = self.pop().await? else {
            return Ok(());
        };
        let Some(EngineCommand { body, reply_to }) = RouterConsumer::decode(bytes) else {
            return Ok(());
        };
        let Some((answer, events)) = self.run(body) else {
            return Ok(());
        };
        // saved first so an event that went out is never lost in a restart
        self.state.save().await;
        if let Some(new_events) = &self.new_events
            && !events.is_empty()
        {
            // only fails when the sender is gone. the events are in the saved
            // history, so they go out after a restart
            let _ = new_events.send(events);
        }
        self.answer(reply_to, answer).await
    }
}
