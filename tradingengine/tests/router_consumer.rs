//! The engine takes its commands from a Redis list, runs them, saves its state, and pushes
//! each answer to the list the command names in `reply_to`, where the router waits for it.
//! These tests play the router's part.
//!
//! They need a running Redis at `REDIS_URL`, so they're `#[ignore]`d. CI starts a Redis and
//! runs them with `-- --ignored`.

mod common;

use common::{HALF_BTC, PRICE_29_000, btc_usd, cancel, limit};
use prost::Message;
use proto::engine_command::Body;
use proto::place_order_request::Amount;
use proto::{
    CancelOrderResponse, ConfigRejectReason, EngineCommand, PlaceOrderRequest, PlaceOrderResponse,
    RejectReason, SetMarketRequest, SetMarketResponse, Side, Uint128,
};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use tradingengine::command_cache::DEFAULT_CACHE_CAPACITY;
use tradingengine::engine::Engine;
use tradingengine::redis_store::RedisStore;
use tradingengine::router_consumer::RouterConsumer;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// The router's part: it pushes commands to one engine's list and reads the answers.
struct Router {
    connection: ConnectionManager,
    /// The list the engine takes its commands from.
    commands_key: String,
    /// The reply keys made so far, to give each command a new one.
    sent: u32,
}

impl Router {
    /// The router for `instance`'s engine. It empties that engine's list first, in case an
    /// earlier run of the test left commands in it.
    async fn new(instance: &str) -> Router {
        let client = redis::Client::open(redis_url()).expect("a valid REDIS_URL");
        let mut connection = ConnectionManager::new(client).await.expect("connect");
        let commands_key = format!("task:pending:{instance}");
        let _: usize = connection.del(&commands_key).await.expect("del");
        Router {
            connection,
            commands_key,
            sent: 0,
        }
    }

    /// Sends a command, and returns the `reply_to` its answer will be pushed to.
    async fn send(&mut self, body: Body) -> String {
        self.sent += 1;
        let reply_to = format!("{}:reply:{}", self.commands_key, self.sent);
        // an answer from an earlier run of the test only expires after a minute
        let _: usize = self.connection.del(&reply_to).await.expect("del");
        let command = EngineCommand {
            body: Some(body),
            reply_to: reply_to.clone(),
        };
        self.push(command.encode_to_vec()).await;
        reply_to
    }

    /// Pushes `bytes` to the engine's list as they are, even if they aren't a command.
    async fn push(&mut self, bytes: Vec<u8>) {
        let _: usize = self
            .connection
            .rpush(&self.commands_key, bytes)
            .await
            .expect("rpush");
    }

    /// The answer that was pushed to `reply_to`, decoded as a `T`.
    async fn answer<T: Message + Default>(&mut self, reply_to: &str) -> T {
        let bytes: Option<Vec<u8>> = self.connection.lpop(reply_to, None).await.expect("lpop");
        let bytes = bytes.expect("the engine pushed an answer to reply_to");
        T::decode(bytes.as_slice()).expect("the answer decodes")
    }
}

/// The router and a new, empty engine for `instance`, which saves to the Redis at
/// `REDIS_URL`.
async fn start(instance: &str) -> (Router, RouterConsumer) {
    let router = Router::new(instance).await;
    let mut store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    // Delete what an earlier run saved. Otherwise an engine that never saves would still
    // pass, because a restart would load the earlier run's state.
    store.clear().await.expect("clear");
    let consumer = RouterConsumer::new(Engine::default(), Some(store), &redis_url(), instance)
        .await
        .expect("connect");
    (router, consumer)
}

/// Sends a command and lets the engine handle it. Returns the `reply_to` of its answer.
async fn run(router: &mut Router, consumer: &mut RouterConsumer, body: Body) -> String {
    let reply_to = router.send(body).await;
    consumer.handle_next().await.expect("the engine handles it");
    reply_to
}

/// The engine a restart of `instance` would load.
async fn restarted(instance: &str) -> Engine {
    let mut store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    let state = store.load_engine_state().await.expect("load");
    Engine::new_from_state(state, DEFAULT_CACHE_CAPACITY).expect("it loads")
}

fn set_market() -> Body {
    Body::SetMarket(SetMarketRequest {
        market: Some(btc_usd(2)),
    })
}

/// A buy of 0.5 BTC at $29,000, with ids from `n`.
fn buy(n: u128) -> PlaceOrderRequest {
    limit(n, Side::Buy, PRICE_29_000, HALF_BTC)
}

// ---- answers ----

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn each_command_is_answered_on_its_reply_to() {
    let (mut router, mut consumer) = start("test-consumer-answers").await;

    let set = run(&mut router, &mut consumer, set_market()).await;
    let set: SetMarketResponse = router.answer(&set).await;
    assert_eq!(
        set.reject_reason(),
        ConfigRejectReason::Unspecified,
        "{}",
        set.reject_detail
    );

    let place = run(&mut router, &mut consumer, Body::PlaceOrder(buy(1))).await;
    let placed: PlaceOrderResponse = router.answer(&place).await;
    assert_eq!(
        placed.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        placed.reject_detail
    );
    assert_eq!(placed.command_id, buy(1).command_id);

    let cancel_key = run(&mut router, &mut consumer, Body::CancelOrder(cancel(1))).await;
    let cancelled: CancelOrderResponse = router.answer(&cancel_key).await;
    assert_eq!(
        cancelled.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        cancelled.reject_detail
    );
    assert_eq!(cancelled.command_id, cancel(1).command_id);
}

/// When the router stops waiting, nobody reads the answer, so it must not stay in Redis.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answer_nobody_reads_expires() {
    let (mut router, mut consumer) = start("test-consumer-expires").await;

    let set = run(&mut router, &mut consumer, set_market()).await;

    let seconds_left: i64 = router.connection.ttl(&set).await.expect("ttl");
    assert!(
        (1..=60).contains(&seconds_left),
        "the answer expires in {seconds_left} s"
    );
}

/// The router adds commands at the end of the list, so the engine must take them from the
/// front. All three are in the list before the engine takes any: if it took the newest
/// first, the cancel would run before its order exists.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn commands_run_in_the_order_they_were_sent() {
    let (mut router, mut consumer) = start("test-consumer-order").await;
    router.send(set_market()).await;
    router.send(Body::PlaceOrder(buy(1))).await;
    let cancel_key = router.send(Body::CancelOrder(cancel(1))).await;

    for _ in 0..3 {
        consumer.handle_next().await.expect("the engine handles it");
    }

    let cancelled: CancelOrderResponse = router.answer(&cancel_key).await;
    assert_eq!(
        cancelled.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        cancelled.reject_detail
    );
}

/// A bad item in the list is skipped. It must not stop the engine, or every command
/// after it would wait forever.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn commands_the_engine_cannot_run_are_skipped() {
    let (mut router, mut consumer) = start("test-consumer-skips").await;
    router.push(vec![0xff, 0xff, 0xff]).await; // not an `EngineCommand`
    let no_body = EngineCommand {
        body: None,
        reply_to: "nobody".to_string(),
    };
    router.push(no_body.encode_to_vec()).await;

    consumer
        .handle_next()
        .await
        .expect("bytes that don't decode are skipped");
    consumer
        .handle_next()
        .await
        .expect("a command without a body is skipped");

    let set = run(&mut router, &mut consumer, set_market()).await;
    let set: SetMarketResponse = router.answer(&set).await;
    assert_eq!(set.reject_reason(), ConfigRejectReason::Unspecified);
}

/// `handle_next` waits up to 5 s for a command. With the redis crate's default response
/// timeout of 500 ms, that wait failed with "timed out", and the engine stopped whenever
/// it had nothing to do.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn waiting_on_an_empty_list_is_not_an_error() {
    let (_router, mut consumer) = start("test-consumer-empty").await;

    consumer
        .handle_next()
        .await
        .expect("an empty list is not an error");
}

// ---- the engine tells the router it runs ----

/// The router asked for two engines. The one that starts moves from `task:to_be_created`
/// to `task:active`; the other one is still waiting.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn a_started_engine_is_active_and_no_longer_waiting() {
    let instance = "test-consumer-registers";
    let other = "test-consumer-registers-other";
    let mut redis = Router::new(instance).await.connection;
    // what an earlier run of the test left
    let _: usize = redis.srem("task:active", instance).await.expect("srem");
    for name in [instance, other] {
        let _: usize = redis
            .lrem("task:to_be_created", 0, name)
            .await
            .expect("lrem");
    }
    let _: usize = redis
        .rpush("task:to_be_created", &[other, instance])
        .await
        .expect("rpush");

    let _started = start(instance).await;

    let active: bool = redis
        .sismember("task:active", instance)
        .await
        .expect("sismember");
    let waiting: Vec<String> = redis
        .lrange("task:to_be_created", 0, -1)
        .await
        .expect("lrange");
    assert!(active, "the engine is in task:active");
    assert!(!waiting.iter().any(|name| name == instance), "{waiting:?}");
    assert!(waiting.iter().any(|name| name == other), "{waiting:?}");
    let _: usize = redis
        .lrem("task:to_be_created", 0, other)
        .await
        .expect("lrem");
}

// ---- the engine saves before it answers ----

/// The contract: nothing leaves the engine until the state it depends on is in Redis. So
/// once an answer is out, a restarted engine loads what that command did.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_command_is_already_saved() {
    let instance = "test-consumer-saves";
    let (mut router, mut consumer) = start(instance).await;
    run(&mut router, &mut consumer, set_market()).await;
    let place = run(&mut router, &mut consumer, Body::PlaceOrder(buy(1))).await;
    let placed: PlaceOrderResponse = router.answer(&place).await;
    assert_eq!(
        placed.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        placed.reject_detail
    );

    let mut restarted = restarted(instance).await;

    let book = restarted
        .market("BTC-USD")
        .expect("the market was saved")
        .book();
    assert_eq!(book.len(), 1);
    // the cache entry was saved too: a retry gets the same answer
    assert_eq!(restarted.place_order(buy(1)), placed);
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_cancel_is_already_saved() {
    let instance = "test-consumer-saves-cancel";
    let (mut router, mut consumer) = start(instance).await;
    run(&mut router, &mut consumer, set_market()).await;
    run(&mut router, &mut consumer, Body::PlaceOrder(buy(1))).await;
    let cancel_key = run(&mut router, &mut consumer, Body::CancelOrder(cancel(1))).await;
    let cancelled: CancelOrderResponse = router.answer(&cancel_key).await;
    assert_eq!(
        cancelled.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        cancelled.reject_detail
    );

    let book_len = restarted(instance)
        .await
        .market("BTC-USD")
        .expect("the market was saved")
        .book()
        .len();
    assert_eq!(book_len, 0);
}

/// A rejection's answer is cached too, so it has to be saved: after a restart, the same
/// command id with a different request is still a DUPLICATE_COMMAND.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_rejection_is_already_saved() {
    let instance = "test-consumer-saves-rejection";
    let (mut router, mut consumer) = start(instance).await;
    run(&mut router, &mut consumer, set_market()).await;
    let mut no_quantity = buy(1);
    no_quantity.amount = Some(Amount::BaseQuantity(Uint128::from(0u128)));
    let place = run(&mut router, &mut consumer, Body::PlaceOrder(no_quantity)).await;
    let rejected: PlaceOrderResponse = router.answer(&place).await;
    assert_eq!(rejected.reject_reason(), RejectReason::BadQuantity);

    // same command id, a valid request this time
    let retry = restarted(instance).await.place_order(buy(1));
    assert_eq!(retry.reject_reason(), RejectReason::DuplicateCommand);
}
