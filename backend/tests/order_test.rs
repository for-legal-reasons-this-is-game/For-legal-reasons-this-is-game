//! Integration tests for placing orders (`advance_order`, the `create_order` endpoint and the order
//! relay), against a real Postgres and TigerBeetle with a fake engine in place of the router.
//! Run them against a throwaway stack like the consumer tests (see consumer/GROUND_TRUTH.md section
//! 13), plus a redis for `AppState` (REDIS_URL, default port 6380 like scripts/integration-test.sh).
//! Every test seeds its own ledgers, users and orders with fresh ids.

use axum::Router;
use axum::routing::{get, post};
use backend::domain::Submission;
use backend::service::EngineClient;
use backend::v1::{self, AppState, advance_order};
use cn_tigerbeetle as tb;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::OnceCell;
use tonic::codegen::tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status};
use uuid::Uuid;

// --- helpers ----------------------------------------------------------------

static MIGRATED: OnceCell<()> = OnceCell::const_new();

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/backend".to_string())
}

async fn pool() -> PgPool {
    let pool = PgPool::connect(&database_url())
        .await
        .expect("connect to postgres");
    MIGRATED
        .get_or_init(|| async {
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("migrations");
        })
        .await;
    pool
}

fn tb_client() -> Arc<tb::Client> {
    let cluster = std::env::var("TB_CLUSTER_ID")
        .map(|v| v.parse().unwrap())
        .unwrap_or(0);
    let addresses =
        std::env::var("TB_IP_ADRESSES").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    Arc::new(tb::Client::new(cluster, &addresses).expect("tigerbeetle client"))
}

async fn redis() -> redis::aio::ConnectionManager {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_string());
    redis::aio::ConnectionManager::new(redis::Client::open(url).expect("redis url"))
        .await
        .expect("connect to redis")
}

fn unique_symbol(prefix: &str) -> String {
    format!("{prefix}{}", &Uuid::new_v4().simple().to_string()[..10]).to_uppercase()
}

const MUST_NOT_OVERDRAW: tb::AccountFlags = tb::AccountFlags::DebitsMustNotExceedCredits;

async fn balance(c: &tb::Client, id: u128) -> (u128, u128) {
    let a = c.lookup_accounts(&[id]).await.expect("lookup").remove(0);
    (a.debits_posted, a.credits_posted)
}

async fn tb_create(c: &tb::Client, account: tb::Account) {
    let r = c.create_accounts(&[account]).await.expect("create account");
    assert!(
        r.is_empty(),
        "account rejected: {:?}",
        r.first().map(|x| &x.result)
    );
}

async fn tb_move(c: &tb::Client, debit: u128, credit: u128, amount: u128, ledger: u32) {
    let r = c
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            debit_account_id: debit,
            credit_account_id: credit,
            amount,
            ledger,
            code: 3,
            ..Default::default()
        }])
        .await
        .expect("create transfer");
    assert!(
        r.is_empty(),
        "transfer rejected: {:?}",
        r.first().map(|x| &x.result)
    );
}

struct World {
    market_id: i32,
    symbol: String,
    base_ledger: i32,
    quote_ledger: i32,
    // tigerbeetle accounts that fund the users of each ledger
    base_source: u128,
    quote_source: u128,
}

// two ledgers (their ledger_id is also the tigerbeetle ledger) with the given decimals and the fee
// account of the quote one. Gives (base ledger, quote ledger, fee account)
async fn ledgers_with_fee(
    pool: &PgPool,
    base_decimals: i16,
    quote_decimals: i16,
) -> (i32, i32, Uuid) {
    let ledger = |tag: &'static str, decimals: i16| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO ledgers (symbol, name, decimals) VALUES ($1, $1, $2) RETURNING ledger_id",
            )
            .bind(unique_symbol(tag))
            .bind(decimals)
            .fetch_one(&pool)
            .await
            .expect("insert ledger")
        }
    };
    let base_ledger = ledger("B", base_decimals).await;
    let quote_ledger = ledger("Q", quote_decimals).await;

    let system_user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id, user_name) VALUES ($1, 'fees')")
        .bind(system_user)
        .execute(pool)
        .await
        .expect("insert fee user");
    let fee_account = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO accounts (account_id, account_name, account_ledger_id, account_code_type, account_user_id, account_status) \
         VALUES ($1, 'fee', $2, 2, $3, 'active')",
    )
    .bind(fee_account)
    .bind(quote_ledger)
    .bind(system_user)
    .execute(pool)
    .await
    .expect("insert fee account");
    (base_ledger, quote_ledger, fee_account)
}

// two ledgers, a fee account and a trading market
async fn world(pool: &PgPool, c: &tb::Client) -> World {
    let (base_ledger, quote_ledger, fee_account) = ledgers_with_fee(pool, 2, 2).await;

    let symbol = unique_symbol("M");
    let market_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO markets (symbol, base_ledger_id, quote_ledger_id, price_decimals, fee_account_id, \
         status, min_base_quantity, min_quote_quantity, maker_fee_bps, taker_fee_bps) \
         VALUES ($1, $2, $3, 2, $4, 'trading', 1, 1, 10, 30) RETURNING market_id",
    )
    .bind(&symbol)
    .bind(base_ledger)
    .bind(quote_ledger)
    .bind(fee_account)
    .fetch_one(pool)
    .await
    .expect("insert market");

    let (base_source, quote_source) = (tb::id(), tb::id());
    for (id, l) in [(base_source, base_ledger), (quote_source, quote_ledger)] {
        tb_create(
            c,
            tb::Account {
                id,
                ledger: l as u32,
                code: 3,
                ..Default::default()
            },
        )
        .await;
    }
    World {
        market_id,
        symbol,
        base_ledger,
        quote_ledger,
        base_source,
        quote_source,
    }
}

struct Party {
    user: Uuid,
    base: Uuid,
    quote: Uuid,
}

const FUNDS: u128 = 1_000_000;

// a user with an active account on each ledger, in postgres and in tigerbeetle, with FUNDS in each
async fn party(pool: &PgPool, c: &tb::Client, w: &World) -> Party {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id, user_name) VALUES ($1, $2)")
        .bind(user)
        .bind(format!("user-{user}"))
        .execute(pool)
        .await
        .expect("insert user");
    let mut accounts = [Uuid::new_v4(), Uuid::new_v4()];
    let sources = [w.base_source, w.quote_source];
    for ((account, ledger), source) in accounts
        .iter_mut()
        .zip([w.base_ledger, w.quote_ledger])
        .zip(sources)
    {
        sqlx::query(
            "INSERT INTO accounts (account_id, account_name, account_ledger_id, account_code_type, account_user_id, account_status) \
             VALUES ($1, 'user', $2, 1, $3, 'active')",
        )
        .bind(*account)
        .bind(ledger)
        .bind(user)
        .execute(pool)
        .await
        .expect("insert account");
        tb_create(
            c,
            tb::Account {
                id: account.as_u128(),
                ledger: ledger as u32,
                code: 1,
                user_data_128: user.as_u128(),
                flags: MUST_NOT_OVERDRAW,
                ..Default::default()
            },
        )
        .await;
        tb_move(c, source, account.as_u128(), FUNDS, ledger as u32).await;
    }
    Party {
        user,
        base: accounts[0],
        quote: accounts[1],
    }
}

// a limit order for 1000 base at 1000, in the state the handler leaves it in after its first
// transaction: `reserving`, with its outbox row. `reserved` is what it has to reserve
async fn insert_order(pool: &PgPool, w: &World, p: &Party, buyer: bool, reserved: u128) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO orders (order_id, idempotency_key, command_id, user_id, market_id, base_account_id, \
         quote_account_id, side, order_type, time_in_force, base_quantity, limit_price, maker_fee_bps, \
         taker_fee_bps, reserved_amount) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 1, 1000, 1000, 10, 30, $9::numeric)",
    )
    .bind(order_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(p.user)
    .bind(w.market_id)
    .bind(p.base)
    .bind(p.quote)
    .bind(if buyer { 1i16 } else { 2i16 })
    .bind(reserved.to_string())
    .execute(pool)
    .await
    .expect("insert order");
    sqlx::query("INSERT INTO order_outbox (order_id) VALUES ($1)")
        .bind(order_id)
        .execute(pool)
        .await
        .expect("insert outbox row");
    order_id
}

// (submission, reject_reason, reject_detail, status)
async fn order_state(
    pool: &PgPool,
    id: Uuid,
) -> (String, Option<i16>, Option<String>, Option<i16>) {
    sqlx::query_as(
        "SELECT submission::text, reject_reason, reject_detail, status FROM orders WHERE order_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read order")
}

async fn command_id_of(pool: &PgPool, id: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT command_id FROM orders WHERE order_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("read command id")
}

// --- a fake engine ---------------------------------------------------------------

use proto::trading_engine_server::{TradingEngine, TradingEngineServer};

// what the fake does with the next PlaceOrder it gets
enum Script {
    Reply(proto::PlaceOrderResponse),
    Fail(Status),
    Slow(Duration), // answers accepted, after a while
    Hang,           // never answers, the caller's timeout has to give up
}

type Calls = Arc<Mutex<Vec<proto::PlaceOrderRequest>>>;

// what the fake does with the next SetMarket it gets (nothing scripted: accepted)
enum MarketScript {
    Reply(i32), // the reject_reason to answer with
    Fail(Status),
    Hang,
}

type MarketCalls = Arc<Mutex<Vec<proto::SetMarketRequest>>>;

struct FakeEngine {
    scripts: Mutex<VecDeque<Script>>,
    calls: Calls,
    market_scripts: Mutex<VecDeque<MarketScript>>,
    market_calls: MarketCalls,
}

#[tonic::async_trait]
impl TradingEngine for FakeEngine {
    async fn set_market(
        &self,
        request: Request<proto::SetMarketRequest>,
    ) -> Result<Response<proto::SetMarketResponse>, Status> {
        self.market_calls.lock().unwrap().push(request.into_inner());
        let script = self.market_scripts.lock().unwrap().pop_front();
        match script {
            None => Ok(Response::new(proto::SetMarketResponse::default())),
            Some(MarketScript::Reply(reject_reason)) => {
                Ok(Response::new(proto::SetMarketResponse {
                    reject_reason,
                    ..Default::default()
                }))
            }
            Some(MarketScript::Fail(status)) => Err(status),
            Some(MarketScript::Hang) => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Err(Status::unavailable("gave up waiting"))
            }
        }
    }
    async fn place_order(
        &self,
        request: Request<proto::PlaceOrderRequest>,
    ) -> Result<Response<proto::PlaceOrderResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        // with nothing scripted the order is accepted
        let script = self.scripts.lock().unwrap().pop_front();
        match script {
            None => Ok(Response::new(accepted())),
            Some(Script::Reply(r)) => Ok(Response::new(r)),
            Some(Script::Fail(status)) => Err(status),
            Some(Script::Slow(wait)) => {
                tokio::time::sleep(wait).await;
                Ok(Response::new(accepted()))
            }
            Some(Script::Hang) => {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Err(Status::unavailable("gave up waiting"))
            }
        }
    }
    async fn cancel_order(
        &self,
        _: Request<proto::CancelOrderRequest>,
    ) -> Result<Response<proto::CancelOrderResponse>, Status> {
        Err(Status::unimplemented("fake"))
    }
    async fn cancel_orders(
        &self,
        _: Request<proto::CancelOrdersRequest>,
    ) -> Result<Response<proto::CancelOrdersResponse>, Status> {
        Err(Status::unimplemented("fake"))
    }
    async fn modify_order(
        &self,
        _: Request<proto::ModifyOrderRequest>,
    ) -> Result<Response<proto::ModifyOrderResponse>, Status> {
        Err(Status::unimplemented("fake"))
    }

    type SubscribeEventsStream = ReceiverStream<Result<proto::EventItem, Status>>;
    async fn subscribe_events(
        &self,
        _: Request<proto::SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        Err(Status::unimplemented("fake"))
    }

    type SubscribeMarketDataStream = ReceiverStream<Result<proto::MarketDataItem, Status>>;
    async fn subscribe_market_data(
        &self,
        _: Request<proto::SubscribeMarketDataRequest>,
    ) -> Result<Response<Self::SubscribeMarketDataStream>, Status> {
        Err(Status::unimplemented("fake"))
    }
}

fn accepted() -> proto::PlaceOrderResponse {
    proto::PlaceOrderResponse {
        status: proto::OrderStatus::New as i32,
        ..Default::default()
    }
}

fn rejected(reason: i32) -> proto::PlaceOrderResponse {
    proto::PlaceOrderResponse {
        reject_reason: reason,
        ..Default::default()
    }
}

// serves the scripted answers (the n-th PlaceOrder gets the n-th script) and gives the client for it
async fn engine(scripts: Vec<Script>) -> (EngineClient, Calls) {
    let calls: Calls = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeEngine {
        scripts: Mutex::new(scripts.into()),
        calls: calls.clone(),
        market_scripts: Mutex::new(VecDeque::new()),
        market_calls: Arc::new(Mutex::new(Vec::new())),
    };
    (serve(fake).await, calls)
}

// the same, for the SetMarket the market endpoint sends
async fn market_engine(scripts: Vec<MarketScript>) -> (EngineClient, MarketCalls) {
    let market_calls: MarketCalls = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeEngine {
        scripts: Mutex::new(VecDeque::new()),
        calls: Arc::new(Mutex::new(Vec::new())),
        market_scripts: Mutex::new(scripts.into()),
        market_calls: market_calls.clone(),
    };
    (serve(fake).await, market_calls)
}

async fn serve(fake: FakeEngine) -> EngineClient {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(TradingEngineServer::new(fake))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    EngineClient::new(&url).expect("engine url")
}

async fn app(pool: &PgPool, c: &Arc<tb::Client>, engine: EngineClient) -> AppState {
    AppState {
        pg_connections: pool.clone(),
        tb_client: c.clone(),
        redis: redis().await,
        engine_client: engine,
    }
}

// --- advance_order -----------------------------------------------------------------

const RESERVED_BUY: u128 = 10_030; // 1000 base at 1000 is 10_000 quote, plus the 30 bps fee

#[tokio::test]
async fn happy_path_reserves_then_accepts() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Accepted);
    assert!(adv.finished);
    assert!(adv.answer.is_some());
    assert_eq!(order_state(&pool, order).await.0, "accepted");
    // the money moved from the user's quote account into the order's hold account
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
    assert_eq!(balance(&c, p.quote.as_u128()).await, (RESERVED_BUY, FUNDS));
    let command_id = command_id_of(&pool, order).await.as_u128();
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command_id, Some(proto::Id::from(command_id)));
}

#[tokio::test]
async fn a_sell_reserves_from_the_base_account() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, false, 1000).await;
    let (eng, _) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(balance(&c, order.as_u128()).await, (0, 1000));
    assert_eq!(balance(&c, p.base.as_u128()).await, (1000, FUNDS));
    assert_eq!(balance(&c, p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn insufficient_funds_rejects_without_calling_the_engine() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, FUNDS + 1).await;
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Rejected);
    assert!(adv.finished); // nothing moved, so nothing is owed back
    let (submission, reason, detail, _) = order_state(&pool, order).await;
    assert_eq!(submission, "rejected");
    assert_eq!(reason, None); // the engine never saw it
    assert!(detail.unwrap_or_default().contains("insufficient funds"));
    assert_eq!(calls.lock().unwrap().len(), 0);
    assert_eq!(balance(&c, p.quote.as_u128()).await, (0, FUNDS));
    assert_eq!(balance(&c, order.as_u128()).await, (0, 0));
}

#[tokio::test]
async fn a_closed_user_account_rejects_instead_of_getting_stuck() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    // close the quote account the way close_account does
    let r = c
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            debit_account_id: p.quote.as_u128(),
            credit_account_id: w.quote_source,
            amount: 0,
            ledger: w.quote_ledger as u32,
            code: 5,
            flags: tb::TransferFlags::Pending | tb::TransferFlags::ClosingDebit,
            ..Default::default()
        }])
        .await
        .expect("close");
    assert!(r.is_empty(), "{:?}", r.first().map(|x| &x.result));
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Rejected);
    assert!(adv.finished);
    assert_eq!(order_state(&pool, order).await.0, "rejected");
    assert_eq!(calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn an_engine_rejection_is_recorded_then_refunded() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, _) = engine(vec![Script::Reply(rejected(1))]).await; // UNKNOWN_MARKET
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Rejected);
    assert!(adv.finished);
    let (submission, reason, _, status) = order_state(&pool, order).await;
    assert_eq!((submission.as_str(), reason), ("rejected", Some(1)));
    assert_eq!(status, None); // only the consumer writes the engine's status
    // everything is back with the user, the hold is empty
    assert_eq!(
        balance(&c, p.quote.as_u128()).await,
        (RESERVED_BUY, FUNDS + RESERVED_BUY)
    );
    assert_eq!(
        balance(&c, order.as_u128()).await,
        (RESERVED_BUY, RESERVED_BUY)
    );
}

#[tokio::test]
async fn a_timeout_leaves_the_order_sent_and_unrefunded() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, calls) = engine(vec![Script::Hang]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Sent);
    assert!(!adv.finished);
    assert_eq!(order_state(&pool, order).await.0, "sent");
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY)); // still reserved
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn retries_send_the_same_request_and_finish_when_the_engine_answers() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, calls) = engine(vec![
        Script::Fail(Status::unavailable("restarting")),
        Script::Fail(Status::unavailable("restarting")),
    ])
    .await;
    let state = app(&pool, &c, eng).await;

    // two attempts inside one call, both fail
    let first = advance_order(&state, order, 2).await.expect("advance");
    assert_eq!(first.submission, Submission::Sent);
    assert!(!first.finished);
    // a later call (the relay's next tick) gets an answer
    let second = advance_order(&state, order, 1).await.expect("advance");
    assert_eq!(second.submission, Submission::Accepted);
    assert!(second.finished);

    // the money was reserved once
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], calls[1]);
    assert_eq!(calls[1], calls[2]); // same command_id and the same sent_at every time
}

#[tokio::test]
async fn an_unknown_reject_number_changes_nothing() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, _) = engine(vec![Script::Reply(rejected(99))]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    // not accepted (the number is not 0) and not refunded
    assert_eq!(adv.submission, Submission::Sent);
    assert!(!adv.finished);
    assert_eq!(order_state(&pool, order).await.0, "sent");
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
}

#[tokio::test]
async fn duplicate_command_changes_nothing() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, _) = engine(vec![Script::Reply(rejected(8))]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Sent);
    assert!(!adv.finished);
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
}

#[tokio::test]
async fn it_resumes_after_a_crash_between_the_hold_account_and_the_reservation() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    // what a crashed run left behind: the hold account exists, nothing is in it
    tb_create(
        &c,
        tb::Account {
            id: order.as_u128(),
            ledger: w.quote_ledger as u32,
            code: 4,
            user_data_128: p.user.as_u128(),
            flags: MUST_NOT_OVERDRAW,
            ..Default::default()
        },
    )
    .await;
    let (eng, _) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    let adv = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(adv.submission, Submission::Accepted);
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
}

#[tokio::test]
async fn advancing_twice_moves_money_once() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    advance_order(&state, order, 1).await.expect("advance");
    let again = advance_order(&state, order, 1).await.expect("advance");

    assert_eq!(again.submission, Submission::Accepted);
    assert!(again.finished);
    assert_eq!(balance(&c, p.quote.as_u128()).await, (RESERVED_BUY, FUNDS));
    assert_eq!(calls.lock().unwrap().len(), 1); // an accepted order is never sent again
}

#[tokio::test]
async fn two_workers_on_one_order_apply_one_reservation_and_one_verdict() {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let (eng, _) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    let (a, b) = tokio::join!(
        advance_order(&state, order, 1),
        advance_order(&state, order, 1)
    );

    assert!(a.expect("first").finished);
    assert!(b.expect("second").finished);
    assert_eq!(order_state(&pool, order).await.0, "accepted");
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
    assert_eq!(balance(&c, p.quote.as_u128()).await, (RESERVED_BUY, FUNDS));
}

// --- POST /users/{user_id}/orders ----------------------------------------------------

// everything a request test needs: a seeded market and user, a fake engine and the handler served
// on a local port
struct Env {
    pool: PgPool,
    c: Arc<tb::Client>,
    w: World,
    p: Party,
    url: String,
    calls: Calls,
}

async fn env(scripts: Vec<Script>) -> Env {
    let (pool, c) = (pool().await, tb_client());
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let (eng, calls) = engine(scripts).await;
    let state = app(&pool, &c, eng).await;
    let router = Router::new()
        .route("/users/{user_id}/orders", post(v1::create_order))
        .route("/orders/{order_id}", get(v1::fetch_order))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Env {
        pool,
        c,
        w,
        p,
        url,
        calls,
    }
}

// 10.00 base at 10.00 (all with 2 decimals): worth 100.00 quote, plus the 30 bps fee = 100.30
fn limit_buy(e: &Env, key: Uuid) -> Value {
    json!({
        "idempotency_key": key.to_string(),
        "market": e.w.symbol,
        "base_account_id": e.p.base.to_string(),
        "quote_account_id": e.p.quote.to_string(),
        "side": "buy",
        "order_type": "limit",
        "time_in_force": "gtc",
        "base_quantity": "10.00",
        "limit_price": "10.00",
    })
}

async fn post_order(e: &Env, user: Uuid, body: &Value) -> (u16, Value) {
    let res = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
        .post(format!("{}/users/{user}/orders", e.url))
        .json(body)
        .send()
        .await
        .expect("send order");
    let status = res.status().as_u16();
    (status, res.json().await.unwrap_or(Value::Null))
}

async fn get_order(e: &Env, order_id: &str) -> Value {
    reqwest::get(format!("{}/orders/{order_id}", e.url))
        .await
        .expect("get order")
        .json()
        .await
        .expect("order json")
}

async fn order_id_for_key(pool: &PgPool, key: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT order_id FROM orders WHERE idempotency_key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .expect("find order")
}

#[tokio::test]
async fn a_limit_buy_returns_201_with_the_reserved_amount() {
    let e = env(vec![]).await;
    let (status, body) = post_order(&e, e.p.user, &limit_buy(&e, Uuid::new_v4())).await;

    assert_eq!(status, 201, "{body}");
    assert_eq!(body["submission"], "accepted");
    assert_eq!(body["reserved_amount"], "100.30");
    assert_eq!(body["status"], "new"); // what the engine answered, not stored
    let order = body["order_id"].as_str().unwrap().parse::<Uuid>().unwrap();
    assert_eq!(balance(&e.c, order.as_u128()).await, (0, 10_030));
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (10_030, FUNDS));
}

#[tokio::test]
async fn a_market_buy_reserves_the_quote_quantity_plus_fee() {
    let e = env(vec![]).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    body["order_type"] = json!("market");
    body["time_in_force"] = json!("ioc");
    body.as_object_mut().unwrap().remove("base_quantity");
    body.as_object_mut().unwrap().remove("limit_price");
    body["quote_quantity"] = json!("100.00");

    let (status, resp) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 201, "{resp}");
    assert_eq!(resp["reserved_amount"], "100.30");
}

#[tokio::test]
async fn a_sell_reserves_only_the_base_quantity() {
    let e = env(vec![]).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    body["side"] = json!("sell");

    let (status, resp) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 201, "{resp}");
    assert_eq!(resp["reserved_amount"], "10.00");
    assert_eq!(balance(&e.c, e.p.base.as_u128()).await, (1000, FUNDS));
}

#[tokio::test]
async fn retrying_an_accepted_order_returns_200_and_the_same_order() {
    let e = env(vec![]).await;
    let body = limit_buy(&e, Uuid::new_v4());
    let (first_status, first) = post_order(&e, e.p.user, &body).await;
    let (second_status, second) = post_order(&e, e.p.user, &body).await;

    assert_eq!((first_status, second_status), (201, 200));
    assert_eq!(first["order_id"], second["order_id"]);
    assert_eq!(e.calls.lock().unwrap().len(), 1);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (10_030, FUNDS));
}

#[tokio::test]
async fn retrying_an_unfinished_order_returns_202_and_reserves_once() {
    // the engine can't be reached for the two attempts of the first request
    let e = env(vec![
        Script::Fail(Status::unavailable("down")),
        Script::Fail(Status::unavailable("down")),
    ])
    .await;
    let body = limit_buy(&e, Uuid::new_v4());
    let (first_status, first) = post_order(&e, e.p.user, &body).await;
    let (second_status, second) = post_order(&e, e.p.user, &body).await;

    assert_eq!(
        (first_status, second_status),
        (202, 202),
        "{first} {second}"
    );
    assert_eq!(first["order_id"], second["order_id"]);
    assert_eq!(second["submission"], "sent");
    // the retry only looked the order up: no second reservation, no third PlaceOrder
    assert_eq!(e.calls.lock().unwrap().len(), 2);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (10_030, FUNDS));
}

#[tokio::test]
async fn reusing_a_key_with_a_different_order_returns_409() {
    let e = env(vec![]).await;
    let key = Uuid::new_v4();
    let (first, _) = post_order(&e, e.p.user, &limit_buy(&e, key)).await;
    let mut other = limit_buy(&e, key);
    other["limit_price"] = json!("11.00");
    let (second, _) = post_order(&e, e.p.user, &other).await;

    assert_eq!((first, second), (201, 409));
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (10_030, FUNDS));
}

#[tokio::test]
async fn not_enough_funds_returns_409_and_the_order_shows_as_rejected() {
    let e = env(vec![]).await;
    let key = Uuid::new_v4();
    let mut body = limit_buy(&e, key);
    body["limit_price"] = json!("10000.00"); // 100_000.00 quote, the user has 10_000.00

    let (status, _) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 409);
    let order = order_id_for_key(&e.pool, key).await.expect("order saved");
    let shown = get_order(&e, &order.to_string()).await;
    assert_eq!(shown["submission"], "rejected");
    assert_eq!(shown["reject_detail"], "insufficient funds");
    assert_eq!(e.calls.lock().unwrap().len(), 0);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn an_engine_rejection_returns_its_code_and_gives_the_money_back() {
    let e = env(vec![Script::Reply(rejected(1))]).await; // UNKNOWN_MARKET
    let (status, _) = post_order(&e, e.p.user, &limit_buy(&e, Uuid::new_v4())).await;

    assert_eq!(status, 404);
    assert_eq!(
        balance(&e.c, e.p.quote.as_u128()).await,
        (10_030, FUNDS + 10_030)
    );
}

#[tokio::test]
async fn the_engine_being_down_returns_202_with_the_order_still_sent() {
    let e = env(vec![
        Script::Fail(Status::unavailable("down")),
        Script::Fail(Status::unavailable("down")),
    ])
    .await;
    let (status, body) = post_order(&e, e.p.user, &limit_buy(&e, Uuid::new_v4())).await;

    assert_eq!(status, 202, "{body}");
    assert_eq!(body["submission"], "sent");
    let order = body["order_id"].as_str().unwrap().parse::<Uuid>().unwrap();
    assert_eq!(balance(&e.c, order.as_u128()).await, (0, 10_030)); // reserved, not refunded
}

#[tokio::test]
async fn a_market_that_is_not_trading_returns_409_and_reserves_nothing() {
    let e = env(vec![]).await;
    sqlx::query("UPDATE markets SET status = 'halted' WHERE market_id = $1")
        .bind(e.w.market_id)
        .execute(&e.pool)
        .await
        .unwrap();
    let key = Uuid::new_v4();

    let (status, _) = post_order(&e, e.p.user, &limit_buy(&e, key)).await;

    assert_eq!(status, 409);
    assert_eq!(order_id_for_key(&e.pool, key).await, None);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn unknown_user_market_and_account_return_404() {
    let e = env(vec![]).await;
    let (status, _) = post_order(&e, Uuid::new_v4(), &limit_buy(&e, Uuid::new_v4())).await;
    assert_eq!(status, 404, "unknown user");

    let mut body = limit_buy(&e, Uuid::new_v4());
    body["market"] = json!("NOPE-NOPE");
    let (status, _) = post_order(&e, e.p.user, &body).await;
    assert_eq!(status, 404, "unknown market");

    let mut body = limit_buy(&e, Uuid::new_v4());
    body["quote_account_id"] = json!(Uuid::new_v4().to_string());
    let (status, _) = post_order(&e, e.p.user, &body).await;
    assert_eq!(status, 404, "unknown account");
}

#[tokio::test]
async fn someone_elses_account_returns_403() {
    let e = env(vec![]).await;
    let other = party(&e.pool, &e.c, &e.w).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    body["quote_account_id"] = json!(other.quote.to_string());

    let (status, _) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 403);
    assert_eq!(balance(&e.c, other.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn an_account_that_is_not_active_returns_409() {
    let e = env(vec![]).await;
    sqlx::query("UPDATE accounts SET account_status = 'processing' WHERE account_id = $1")
        .bind(e.p.quote)
        .execute(&e.pool)
        .await
        .unwrap();

    let (status, _) = post_order(&e, e.p.user, &limit_buy(&e, Uuid::new_v4())).await;

    assert_eq!(status, 409);
}

#[tokio::test]
async fn an_account_on_the_wrong_ledger_returns_400() {
    let e = env(vec![]).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    // the base account where the quote account goes
    body["quote_account_id"] = json!(e.p.base.to_string());

    let (status, _) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 400);
}

// a fresh limit buy with one field changed, the status it gets
async fn status_with(e: &Env, field: &str, value: &str) -> u16 {
    let mut body = limit_buy(e, Uuid::new_v4());
    body[field] = json!(value);
    post_order(e, e.p.user, &body).await.0
}

#[tokio::test]
async fn amounts_that_cannot_be_traded_return_400() {
    let e = env(vec![]).await;
    // more decimals than the ledger has
    assert_eq!(status_with(&e, "base_quantity", "10.001").await, 400);
    // doesn't fit anywhere
    assert_eq!(
        status_with(&e, "base_quantity", "79228162514264337593543950335").await,
        400
    );
    // zero and negative
    assert_eq!(status_with(&e, "base_quantity", "0").await, 400);
    assert_eq!(status_with(&e, "limit_price", "-1.00").await, 400);

    // 0.01 base at 0.01 is worth less than the smallest quote unit
    let mut tiny = limit_buy(&e, Uuid::new_v4());
    tiny["base_quantity"] = json!("0.01");
    tiny["limit_price"] = json!("0.01");
    assert_eq!(post_order(&e, e.p.user, &tiny).await.0, 400);

    // nothing was reserved by any of them
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn a_wrong_combination_of_fields_returns_400() {
    let e = env(vec![]).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    body.as_object_mut().unwrap().remove("limit_price"); // a limit order needs one

    let (status, _) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 400);
}

#[tokio::test]
async fn a_past_or_malformed_expire_time_returns_400() {
    let e = env(vec![]).await;
    for expire in ["2000-01-01T00:00:00Z", "not a date"] {
        let mut body = limit_buy(&e, Uuid::new_v4());
        body["time_in_force"] = json!("gtd");
        body["expire_time"] = json!(expire);
        let (status, _) = post_order(&e, e.p.user, &body).await;
        assert_eq!(status, 400, "{expire}");
    }
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn a_gtd_order_in_the_future_is_sent_with_its_expiry() {
    let e = env(vec![]).await;
    let mut body = limit_buy(&e, Uuid::new_v4());
    body["time_in_force"] = json!("gtd");
    body["expire_time"] = json!("2999-01-01T00:00:00Z");

    let (status, resp) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 201, "{resp}");
    let calls = e.calls.lock().unwrap();
    let expire = calls[0].expire_time.as_ref().expect("expire_time sent");
    assert_eq!(expire.seconds, 32_472_144_000); // 2999-01-01T00:00:00Z
}

// --- the order relay ------------------------------------------------------------------

use backend::relay::order_relay_tick;

// the relay works on every pending order there is, so the relay tests take turns and each starts
// with an empty queue
static RELAY_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn relay_turn(pool: &PgPool) -> tokio::sync::MutexGuard<'static, ()> {
    let turn = RELAY_TESTS.lock().await;
    sqlx::query("UPDATE order_outbox SET processed_at = now() WHERE processed_at IS NULL")
        .execute(pool)
        .await
        .expect("empty the queue");
    turn
}

// makes the order's outbox row old enough for the relay (it leaves a grace period for the handler)
async fn age_outbox(pool: &PgPool, order: Uuid) {
    sqlx::query(
        "UPDATE order_outbox SET created_at = now() - interval '1 minute' WHERE order_id = $1",
    )
    .bind(order)
    .execute(pool)
    .await
    .expect("age outbox row");
}

async fn outbox_done(pool: &PgPool, order: Uuid) -> bool {
    sqlx::query_scalar("SELECT processed_at IS NOT NULL FROM order_outbox WHERE order_id = $1")
        .bind(order)
        .fetch_one(pool)
        .await
        .expect("read outbox row")
}

#[tokio::test]
async fn the_relay_finishes_an_order_left_in_reserving() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    age_outbox(&pool, order).await;
    let (eng, _) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    order_relay_tick(&state).await;

    assert_eq!(order_state(&pool, order).await.0, "accepted");
    assert!(outbox_done(&pool, order).await);
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
}

#[tokio::test]
async fn the_relay_leaves_orders_inside_the_grace_period_to_the_handler() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await; // just created
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    order_relay_tick(&state).await;

    assert_eq!(order_state(&pool, order).await.0, "reserving");
    assert!(!outbox_done(&pool, order).await);
    assert_eq!(calls.lock().unwrap().len(), 0);
    assert_eq!(balance(&c, p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn the_relay_gives_back_the_money_of_a_rejection_that_was_only_recorded() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    // reserve it, with the engine unreachable
    let (eng, _) = engine(vec![Script::Fail(Status::unavailable("down"))]).await;
    let state = app(&pool, &c, eng).await;
    let adv = advance_order(&state, order, 1).await.expect("advance");
    assert_eq!(adv.submission, Submission::Sent);
    // what a crash right after the engine's rejection was saved leaves behind
    sqlx::query("UPDATE orders SET submission = 'rejected', reject_reason = 1 WHERE order_id = $1")
        .bind(order)
        .execute(&pool)
        .await
        .unwrap();
    age_outbox(&pool, order).await;

    order_relay_tick(&state).await;

    assert!(outbox_done(&pool, order).await);
    assert_eq!(
        balance(&c, p.quote.as_u128()).await,
        (RESERVED_BUY, FUNDS + RESERVED_BUY)
    );
}

#[tokio::test]
async fn the_relay_keeps_retrying_an_order_the_engine_never_answers() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    let down = || Script::Fail(Status::unavailable("down"));
    let (eng, calls) = engine(vec![down(), down(), down()]).await;
    let state = app(&pool, &c, eng).await;

    for _ in 0..3 {
        age_outbox(&pool, order).await;
        order_relay_tick(&state).await;
    }

    // still waiting, one attempt per tick, and the money was never given back
    assert_eq!(order_state(&pool, order).await.0, "sent");
    assert!(!outbox_done(&pool, order).await);
    assert_eq!(calls.lock().unwrap().len(), 3);
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));

    // and when the engine is back the next tick finishes it
    order_relay_tick(&state).await;
    assert_eq!(order_state(&pool, order).await.0, "accepted");
    assert!(outbox_done(&pool, order).await);
}

#[tokio::test]
async fn two_relays_do_not_apply_an_order_twice() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    age_outbox(&pool, order).await;
    let (eng, calls) = engine(vec![]).await;
    let state = app(&pool, &c, eng).await;

    tokio::join!(order_relay_tick(&state), order_relay_tick(&state));

    assert_eq!(order_state(&pool, order).await.0, "accepted");
    assert!(outbox_done(&pool, order).await);
    assert_eq!(balance(&c, order.as_u128()).await, (0, RESERVED_BUY));
    assert_eq!(calls.lock().unwrap().len(), 1); // the second relay skipped the locked row
}

#[tokio::test]
async fn stuck_orders_do_not_starve_the_ones_behind_them() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    // three orders the engine answers with a number nobody knows: they never finish, so the relay
    // tries them on every pass. A fourth order comes after them
    let mut stuck = Vec::new();
    for _ in 0..3 {
        stuck.push(insert_order(&pool, &w, &p, true, RESERVED_BUY).await);
    }
    let good = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
    for order in stuck.iter().chain([&good]) {
        age_outbox(&pool, *order).await;
    }
    let scripts = (0..3).map(|_| Script::Reply(rejected(99))).collect();
    let (eng, _) = engine(scripts).await;
    let state = app(&pool, &c, eng).await;

    order_relay_tick(&state).await; // takes the three oldest, none of them finishes
    assert_eq!(order_state(&pool, stuck[0]).await.0, "sent");
    assert_eq!(order_state(&pool, good).await.0, "reserving"); // not reached yet

    order_relay_tick(&state).await; // has to move on to the one it hasn't tried yet
    assert_eq!(order_state(&pool, good).await.0, "accepted");
    assert!(outbox_done(&pool, good).await);
}

#[tokio::test]
async fn the_relay_takes_a_few_orders_per_pass() {
    let (pool, c) = (pool().await, tb_client());
    let _turn = relay_turn(&pool).await;
    let w = world(&pool, &c).await;
    let p = party(&pool, &c, &w).await;
    let mut orders = Vec::new();
    for _ in 0..5 {
        let order = insert_order(&pool, &w, &p, true, RESERVED_BUY).await;
        age_outbox(&pool, order).await;
        orders.push(order);
    }
    let down = || Script::Fail(Status::unavailable("down"));
    let (eng, calls) = engine((0..5).map(|_| down()).collect()).await;
    let state = app(&pool, &c, eng).await;

    order_relay_tick(&state).await;

    // a pass holds a database connection while it waits on the network, so it stays short
    let tried: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM order_outbox WHERE order_id = ANY($1) AND last_attempt_at IS NOT NULL",
    )
    .bind(&orders)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(tried, 3);
    assert_eq!(calls.lock().unwrap().len(), 3);
}

// --- fixes from the final review ----------------------------------------------------------

#[tokio::test]
async fn a_reservation_too_big_to_be_read_back_returns_400() {
    let e = env(vec![]).await;
    let key = Uuid::new_v4();
    // 1e15 base at 1e13 is worth 1e30 smallest quote units: fits a u128 and the database, but not
    // the decimal type the order is read back with, which would break reading the order lists
    let mut body = limit_buy(&e, key);
    body["base_quantity"] = json!("1000000000000000.00");
    body["limit_price"] = json!("10000000000000.00");

    let (status, _) = post_order(&e, e.p.user, &body).await;

    assert_eq!(status, 400);
    assert_eq!(order_id_for_key(&e.pool, key).await, None);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn expire_times_postgres_reads_but_we_cannot_use_return_400() {
    let e = env(vec![]).await;
    for expire in ["infinity", "294276-01-01T00:00:00Z", "\u{0}"] {
        let key = Uuid::new_v4();
        let mut body = limit_buy(&e, key);
        body["time_in_force"] = json!("gtd");
        body["expire_time"] = json!(expire);
        let (status, _) = post_order(&e, e.p.user, &body).await;
        assert_eq!(status, 400, "{expire:?}");
        assert_eq!(order_id_for_key(&e.pool, key).await, None, "{expire:?}");
    }
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn a_retry_after_the_market_was_halted_still_returns_the_order() {
    let e = env(vec![]).await;
    let body = limit_buy(&e, Uuid::new_v4());
    let (first_status, first) = post_order(&e, e.p.user, &body).await;
    sqlx::query("UPDATE markets SET status = 'halted' WHERE market_id = $1")
        .bind(e.w.market_id)
        .execute(&e.pool)
        .await
        .unwrap();

    let (retry_status, retry) = post_order(&e, e.p.user, &body).await;
    let mut other = body.clone();
    other["limit_price"] = json!("11.00");
    let (other_status, _) = post_order(&e, e.p.user, &other).await;

    assert_eq!((first_status, retry_status), (201, 200));
    assert_eq!(first["order_id"], retry["order_id"]);
    assert_eq!(other_status, 409); // a different order under the key is still a mistake
    // a new order on the halted market is still refused
    let (new_status, _) = post_order(&e, e.p.user, &limit_buy(&e, Uuid::new_v4())).await;
    assert_eq!(new_status, 409);
}

#[tokio::test]
async fn a_negative_fee_rate_is_refused_before_anything_is_reserved() {
    let e = env(vec![]).await;
    sqlx::query("UPDATE markets SET maker_fee_bps = -1 WHERE market_id = $1")
        .bind(e.w.market_id)
        .execute(&e.pool)
        .await
        .unwrap();
    let key = Uuid::new_v4();

    let (status, _) = post_order(&e, e.p.user, &limit_buy(&e, key)).await;

    assert_eq!(status, 500); // the market is broken, the request is fine
    assert_eq!(order_id_for_key(&e.pool, key).await, None);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (0, FUNDS));
}

#[tokio::test]
async fn two_simultaneous_requests_with_one_key_place_one_order() {
    // the engine takes a while, so the second request arrives while the first is still working
    let e = env(vec![Script::Slow(Duration::from_millis(500))]).await;
    let key = Uuid::new_v4();
    let body = limit_buy(&e, key);

    let ((status_a, a), (status_b, b)) = tokio::join!(
        post_order(&e, e.p.user, &body),
        post_order(&e, e.p.user, &body)
    );

    assert!(
        [status_a, status_b]
            .iter()
            .all(|s| [200, 201, 202].contains(s))
    );
    assert_eq!(a["order_id"], b["order_id"]);
    let orders: i64 = sqlx::query_scalar("SELECT count(*) FROM orders WHERE idempotency_key = $1")
        .bind(key)
        .fetch_one(&e.pool)
        .await
        .unwrap();
    assert_eq!(orders, 1);
    assert_eq!(balance(&e.c, e.p.quote.as_u128()).await, (10_030, FUNDS));
    assert_eq!(e.calls.lock().unwrap().len(), 1);
}

// --- create_market -----------------------------------------------------------------

struct MarketEnv {
    pool: PgPool,
    state: AppState,
    url: String,
    calls: MarketCalls,
    base_symbol: String,
    quote_symbol: String,
    quote_ledger: i32,
    fee_account: Uuid,
}

// two ledgers (base with 3 decimals, quote with 2) with the quote one's fee account, and the
// endpoint served with a fake engine that answers SetMarket as scripted
async fn market_env(scripts: Vec<MarketScript>) -> MarketEnv {
    let (pool, c) = (pool().await, tb_client());
    let (base_ledger, quote_ledger, fee_account) = ledgers_with_fee(&pool, 3, 2).await;
    let symbol_of = |id: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, String>("SELECT symbol FROM ledgers WHERE ledger_id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("ledger symbol")
        }
    };
    let (base_symbol, quote_symbol) = (symbol_of(base_ledger).await, symbol_of(quote_ledger).await);
    let (eng, calls) = market_engine(scripts).await;
    let state = app(&pool, &c, eng).await;
    let router = Router::new()
        .route("/markets", post(v1::create_market))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    MarketEnv {
        pool,
        state,
        url,
        calls,
        base_symbol,
        quote_symbol,
        quote_ledger,
        fee_account,
    }
}

// base has 3 decimals and quote 2: minimum 0.005 base is 5, minimum 1.50 quote is 150, steps 5 and 5
fn market_body(e: &MarketEnv, symbol: &str) -> Value {
    json!({
        "symbol": symbol,
        "base_ledger_symbol": e.base_symbol,
        "quote_ledger_symbol": e.quote_symbol,
        "price_decimals": 4,
        "min_base_quantity": "0.005",
        "min_quote_quantity": "1.50",
        "quantity_step": "0.005",
        "price_step": "0.0005",
        "maker_fee_bps": 10,
        "taker_fee_bps": 30,
    })
}

async fn post_market(e: &MarketEnv, body: &Value) -> (u16, Value) {
    let res = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
        .post(format!("{}/markets", e.url))
        .json(body)
        .send()
        .await
        .expect("send market");
    let status = res.status().as_u16();
    (status, res.json().await.unwrap_or(Value::Null))
}

async fn count_where(pool: &PgPool, sql: &'static str, symbol: &str) -> i64 {
    sqlx::query_scalar(sql)
        .bind(symbol)
        .fetch_one(pool)
        .await
        .expect("count")
}

const MARKETS_WITH_SYMBOL: &str = "SELECT count(*) FROM markets WHERE symbol = $1";
const PENDING_OUTBOX_OF_SYMBOL: &str = "SELECT count(*) FROM market_outbox o \
     JOIN markets m ON m.market_id = o.market_id WHERE m.symbol = $1 AND o.processed_at IS NULL";

#[tokio::test]
async fn create_market_inserts_a_halted_market_and_a_pending_outbox_row() {
    // the engine can't be reached, so nothing but the insert has happened
    let e = market_env(vec![MarketScript::Fail(Status::unavailable("down"))]).await;
    let symbol = unique_symbol("M");

    let (status, body) = post_market(&e, &market_body(&e, &symbol)).await;

    assert_eq!(status, 202);
    assert_eq!(body["status"], "halted");
    assert_eq!(body["symbol"], symbol.as_str());
    let raw: (String, String, String, String, i32, i32) = sqlx::query_as(
        "SELECT min_base_quantity::text, min_quote_quantity::text, quantity_step::text, \
                price_step::text, maker_fee_bps, taker_fee_bps FROM markets WHERE symbol = $1",
    )
    .bind(&symbol)
    .fetch_one(&e.pool)
    .await
    .expect("stored market");
    assert_eq!(
        raw,
        ("5".into(), "150".into(), "5".into(), "5".into(), 10, 30)
    );
    assert_eq!(
        count_where(&e.pool, PENDING_OUTBOX_OF_SYMBOL, &symbol).await,
        1
    );
}

#[tokio::test]
async fn create_market_rejects_bad_requests_with_the_right_status() {
    let e = market_env(vec![]).await;
    let symbol = unique_symbol("M");
    let with = |key: &str, value: Value| {
        let mut body = market_body(&e, &symbol);
        body[key] = value;
        body
    };
    let cases = [
        (with("base_ledger_symbol", json!("NOPE")), 404),
        (with("quote_ledger_symbol", json!("NOPE")), 404),
        (with("quote_ledger_symbol", json!(e.base_symbol)), 400),
        (with("symbol", json!("")), 400),
        (with("symbol", json!("   ")), 400),
        (with("min_base_quantity", json!("0")), 400),
        (with("min_base_quantity", json!("-1")), 400),
        (with("min_base_quantity", json!("0.0005")), 400), // finer than the 3 decimals of base
        (with("min_quote_quantity", json!("0")), 400),
        (with("quantity_step", json!("0")), 400),
        (with("price_step", json!("0")), 400),
        (with("price_decimals", json!(-1)), 400),
        (with("price_decimals", json!(19)), 400),
        (with("maker_fee_bps", json!(-1)), 400),
        (with("taker_fee_bps", json!(10001)), 400),
    ];
    for (body, expected) in cases {
        let (status, _) = post_market(&e, &body).await;
        assert_eq!(status, expected, "body: {body}");
    }

    sqlx::query("UPDATE ledgers SET enabled = false WHERE ledger_id = $1")
        .bind(e.quote_ledger)
        .execute(&e.pool)
        .await
        .expect("disable the ledger");
    let (status, _) = post_market(&e, &market_body(&e, &symbol)).await;
    assert_eq!(status, 400, "disabled ledger");

    assert_eq!(count_where(&e.pool, MARKETS_WITH_SYMBOL, &symbol).await, 0);
    assert!(e.calls.lock().unwrap().is_empty(), "the engine was called");
}

#[tokio::test]
async fn create_market_conflicts_on_duplicate_symbol_or_pair() {
    let e = market_env(vec![MarketScript::Fail(Status::unavailable("down"))]).await;
    let symbol = unique_symbol("M");
    let body = market_body(&e, &symbol);

    assert_eq!(post_market(&e, &body).await.0, 202);
    // re-posting after a 202 is a conflict, nothing is duplicated
    assert_eq!(post_market(&e, &body).await.0, 409);
    // another symbol for the same ledger pair
    let (status, _) = post_market(&e, &market_body(&e, &unique_symbol("N"))).await;
    assert_eq!(status, 409);

    assert_eq!(count_where(&e.pool, MARKETS_WITH_SYMBOL, &symbol).await, 1);
    assert_eq!(
        count_where(&e.pool, PENDING_OUTBOX_OF_SYMBOL, &symbol).await,
        1
    );
}

async fn market_state(e: &MarketEnv, symbol: &str) -> (String, bool) {
    sqlx::query_as(
        "SELECT m.status::text, o.processed_at IS NOT NULL FROM markets m \
         JOIN market_outbox o ON o.market_id = m.market_id WHERE m.symbol = $1",
    )
    .bind(symbol)
    .fetch_one(&e.pool)
    .await
    .expect("market state")
}

#[tokio::test]
async fn create_market_activates_when_the_engine_accepts() {
    let e = market_env(vec![]).await;
    let symbol = unique_symbol("M");

    let (status, body) = post_market(&e, &market_body(&e, &symbol)).await;

    assert_eq!(status, 201);
    assert_eq!(body["status"], "trading");
    assert_eq!(
        market_state(&e, &symbol).await,
        ("trading".to_string(), true)
    );
    let calls = e.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let sent = calls[0].market.as_ref().expect("market in the request");
    assert_eq!(sent.symbol, symbol);
    assert_eq!(
        (sent.base_decimals, sent.quote_decimals, sent.price_decimals),
        (3, 2, 4)
    );
    assert_eq!(sent.fee_account, Some(e.fee_account.as_u128().into()));
}

#[tokio::test]
async fn create_market_stays_halted_when_the_engine_cannot_be_reached() {
    for script in [
        MarketScript::Fail(Status::unavailable("down")),
        MarketScript::Hang,
    ] {
        let e = market_env(vec![script]).await;
        let symbol = unique_symbol("M");

        let (status, body) = post_market(&e, &market_body(&e, &symbol)).await;

        assert_eq!(status, 202);
        assert_eq!(body["status"], "halted");
        assert_eq!(
            market_state(&e, &symbol).await,
            ("halted".to_string(), false)
        );
        assert_eq!(e.calls.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn create_market_refused_by_the_engine_is_removed() {
    let e = market_env(vec![MarketScript::Reply(1)]).await; // BAD_DEFINITION
    let symbol = unique_symbol("M");

    let (status, _) = post_market(&e, &market_body(&e, &symbol)).await;

    assert_eq!(status, 400);
    assert_eq!(count_where(&e.pool, MARKETS_WITH_SYMBOL, &symbol).await, 0);
    // nothing is left behind, the same symbol can be created again
    assert_eq!(post_market(&e, &market_body(&e, &symbol)).await.0, 201);
}

#[tokio::test]
async fn create_market_answers_that_are_not_accepted_never_activate() {
    // HAS_OPEN_ORDERS and a number this backend doesn't know
    for reason in [2, 99] {
        let e = market_env(vec![MarketScript::Reply(reason)]).await;
        let symbol = unique_symbol("M");

        let (status, body) = post_market(&e, &market_body(&e, &symbol)).await;

        assert_eq!(status, 202);
        assert_eq!(body["status"], "halted");
        assert_eq!(
            market_state(&e, &symbol).await,
            ("halted".to_string(), false)
        );
    }
}

#[tokio::test]
async fn activate_market_does_nothing_for_a_finished_row() {
    let e = market_env(vec![]).await;
    let symbol = unique_symbol("M");
    assert_eq!(post_market(&e, &market_body(&e, &symbol)).await.0, 201);
    // someone halts it afterwards; a repeat of the activation must not turn it back on
    sqlx::query("UPDATE markets SET status = 'halted' WHERE symbol = $1")
        .bind(&symbol)
        .execute(&e.pool)
        .await
        .expect("halt");
    let market_id: i32 = sqlx::query_scalar("SELECT market_id FROM markets WHERE symbol = $1")
        .bind(&symbol)
        .fetch_one(&e.pool)
        .await
        .expect("market id");

    v1::activate_market(&e.pool, market_id)
        .await
        .expect("activate");

    assert_eq!(
        market_state(&e, &symbol).await,
        ("halted".to_string(), true)
    );
}

// --- the market relay ---------------------------------------------------------------

use backend::relay::market_relay_tick;

// like relay_turn: the relay works on every aged pending market there is, so these tests take turns
// and each starts without leftovers (only the old ones, the fresh rows belong to tests still running)
async fn market_relay_turn(pool: &PgPool) -> tokio::sync::MutexGuard<'static, ()> {
    let turn = RELAY_TESTS.lock().await;
    sqlx::query(
        "UPDATE market_outbox SET processed_at = now() \
         WHERE processed_at IS NULL AND created_at < now() - interval '15 seconds'",
    )
    .execute(pool)
    .await
    .expect("empty the queue");
    turn
}

// makes the outbox row old enough for the relay (it leaves a grace period for the handler)
async fn age_market_outbox(pool: &PgPool, symbol: &str) {
    sqlx::query(
        "UPDATE market_outbox SET created_at = now() - interval '1 minute' \
         WHERE market_id = (SELECT market_id FROM markets WHERE symbol = $1)",
    )
    .bind(symbol)
    .execute(pool)
    .await
    .expect("age outbox row");
}

#[tokio::test]
async fn the_market_relay_finishes_a_market_the_handler_could_not() {
    let e = market_env(vec![MarketScript::Fail(Status::unavailable("down"))]).await;
    let _turn = market_relay_turn(&e.pool).await;
    let symbol = unique_symbol("M");
    assert_eq!(post_market(&e, &market_body(&e, &symbol)).await.0, 202);
    age_market_outbox(&e.pool, &symbol).await;

    market_relay_tick(&e.state).await;

    assert_eq!(
        market_state(&e, &symbol).await,
        ("trading".to_string(), true)
    );
    assert_eq!(e.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn the_market_relay_leaves_markets_inside_the_grace_period_to_the_handler() {
    let e = market_env(vec![MarketScript::Fail(Status::unavailable("down"))]).await;
    let _turn = market_relay_turn(&e.pool).await;
    let symbol = unique_symbol("M");
    assert_eq!(post_market(&e, &market_body(&e, &symbol)).await.0, 202);

    market_relay_tick(&e.state).await;

    assert_eq!(
        market_state(&e, &symbol).await,
        ("halted".to_string(), false)
    );
    assert_eq!(e.calls.lock().unwrap().len(), 1);
}

// a halted market with its own ledger pair and an old, pending outbox row. Gives its market_id
async fn pending_market(pool: &PgPool) -> i32 {
    let (base_ledger, quote_ledger, fee_account) = ledgers_with_fee(pool, 2, 2).await;
    sqlx::query_scalar::<_, i32>(
        "WITH m AS ( \
             INSERT INTO markets (symbol, base_ledger_id, quote_ledger_id, price_decimals, \
                                  fee_account_id, min_base_quantity, min_quote_quantity, \
                                  maker_fee_bps, taker_fee_bps) \
             VALUES ($1, $2, $3, 2, $4, 1, 1, 10, 30) RETURNING market_id \
         ), o AS ( \
             INSERT INTO market_outbox (market_id, created_at) \
             SELECT market_id, now() - interval '1 minute' FROM m \
         ) SELECT market_id FROM m",
    )
    .bind(unique_symbol("M"))
    .bind(base_ledger)
    .bind(quote_ledger)
    .bind(fee_account)
    .fetch_one(pool)
    .await
    .expect("pending market")
}

#[tokio::test]
async fn the_market_relay_goes_round_the_pending_markets() {
    let pool = pool().await;
    let _turn = market_relay_turn(&pool).await;
    // the first three attempts fail, so those three stay pending
    let down = || MarketScript::Fail(Status::unavailable("down"));
    let (eng, calls) = market_engine(vec![down(), down(), down()]).await;
    let state = app(&pool, &tb_client(), eng).await;
    let mut markets = Vec::new();
    for _ in 0..4 {
        markets.push(pending_market(&pool).await);
    }
    let trading = |id: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT status = 'trading' FROM markets WHERE market_id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("market status")
        }
    };

    market_relay_tick(&state).await;
    assert_eq!(calls.lock().unwrap().len(), 3);
    for id in &markets {
        assert!(!trading(*id).await);
    }

    // the one that was never tried goes first, before the three that just failed
    market_relay_tick(&state).await;
    assert!(trading(markets[3]).await);
}

#[tokio::test]
async fn the_market_relay_removes_a_market_the_engine_refuses() {
    let pool = pool().await;
    let _turn = market_relay_turn(&pool).await;
    let (eng, calls) = market_engine(vec![MarketScript::Reply(1)]).await; // BAD_DEFINITION
    let state = app(&pool, &tb_client(), eng).await;
    let market_id = pending_market(&pool).await;

    market_relay_tick(&state).await;

    let left: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM markets WHERE market_id = $1) \
              + (SELECT count(*) FROM market_outbox WHERE market_id = $1)",
    )
    .bind(market_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(left, 0);
    // and it is not asked again
    market_relay_tick(&state).await;
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn create_market_rejects_decimals_that_cannot_price_an_order() {
    let e = market_env(vec![]).await;
    let symbol = unique_symbol("M");
    // the quote of an order is price * base / 10^(price_decimals + base_decimals - quote_decimals):
    // with base 3 and price 4 decimals, a quote ledger with 9 gives a negative exponent
    sqlx::query("UPDATE ledgers SET decimals = 9 WHERE ledger_id = $1")
        .bind(e.quote_ledger)
        .execute(&e.pool)
        .await
        .expect("change decimals");

    let (status, _) = post_market(&e, &market_body(&e, &symbol)).await;

    assert_eq!(status, 400);
    assert_eq!(count_where(&e.pool, MARKETS_WITH_SYMBOL, &symbol).await, 0);
    assert!(e.calls.lock().unwrap().is_empty());
}
