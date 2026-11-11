//! Integration tests for the event consumer, against a real Postgres (and TigerBeetle).
//! Run them through `scripts/integration-test.sh`, or point `DATABASE_URL` / `TB_*` at a
//! throwaway stack. Every test seeds its own ledgers, users and orders with fresh ids.

use backend::consumer::ledger::{self, LedgerError};
use backend::consumer::lock;
use backend::consumer::plan::{OrderUpdate, TradeInsert};
use backend::consumer::store;
use backend::consumer::{self, Config, ConsumerError};
use backend::domain::TransferCodeType;
use cn_tigerbeetle as tb;
use sqlx::PgPool;
use std::time::{Duration, SystemTime};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// --- helpers ----------------------------------------------------------------

static MIGRATED: OnceCell<()> = OnceCell::const_new();

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/backend".to_string())
}

async fn pool() -> PgPool {
    let url = database_url();
    let pool = PgPool::connect(&url).await.expect("connect to postgres");
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

fn unique_symbol(prefix: &str) -> String {
    format!("{prefix}{}", &Uuid::new_v4().simple().to_string()[..10]).to_uppercase()
}

struct World {
    fee_account: Uuid,
    market_id: i32,
    symbol: String,
    base_ledger: i32,
    quote_ledger: i32,
}

async fn world(pool: &PgPool) -> World {
    let ledger = |tag: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i32>(
                "INSERT INTO ledgers (symbol, name, decimals) VALUES ($1, $1, 2) RETURNING ledger_id",
            )
            .bind(unique_symbol(tag))
            .fetch_one(&pool)
            .await
            .expect("insert ledger")
        }
    };
    let base_ledger = ledger("B").await;
    let quote_ledger = ledger("Q").await;

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

    let symbol = unique_symbol("M");
    let market_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO markets (symbol, base_ledger_id, quote_ledger_id, price_decimals, fee_account_id, \
         min_base_quantity, min_quote_quantity, maker_fee_bps, taker_fee_bps) \
         VALUES ($1, $2, $3, 2, $4, 1, 1, 10, 30) RETURNING market_id",
    )
    .bind(&symbol)
    .bind(base_ledger)
    .bind(quote_ledger)
    .bind(fee_account)
    .fetch_one(pool)
    .await
    .expect("insert market");
    World {
        fee_account,
        market_id,
        symbol,
        base_ledger,
        quote_ledger,
    }
}

struct Party {
    user: Uuid,
    base: Uuid,
    quote: Uuid,
}

async fn party(pool: &PgPool, w: &World) -> Party {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users (user_id, user_name) VALUES ($1, $2)")
        .bind(user)
        .bind(format!("user-{user}"))
        .execute(pool)
        .await
        .expect("insert user");
    let mut accounts = [Uuid::new_v4(), Uuid::new_v4()];
    for (account, ledger) in accounts.iter_mut().zip([w.base_ledger, w.quote_ledger]) {
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
    }
    Party {
        user,
        base: accounts[0],
        quote: accounts[1],
    }
}

/// A limit order with limit price 1000, and `status` still NULL, as before any engine report.
async fn order(pool: &PgPool, w: &World, p: &Party, buyer: bool) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO orders (order_id, idempotency_key, command_id, user_id, market_id, base_account_id, \
         quote_account_id, side, order_type, time_in_force, base_quantity, limit_price, maker_fee_bps, \
         taker_fee_bps, reserved_amount) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 1, 1000, 1000, 10, 30, 1000000)",
    )
    .bind(order_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(p.user)
    .bind(w.market_id)
    .bind(p.base)
    .bind(p.quote)
    .bind(if buyer { 1i16 } else { 2i16 })
    .execute(pool)
    .await
    .expect("insert order");
    order_id
}

fn trade(w: &World, buyer_order: Uuid, seller_order: Uuid) -> TradeInsert {
    TradeInsert {
        trade_id: Uuid::new_v4(),
        market_id: w.market_id,
        buyer_order_id: buyer_order,
        seller_order_id: seller_order,
        price: 1000,
        base_quantity: 10,
        quote_quantity: 10_000,
        buyer_fee: 30,
        seller_fee: 10,
        taker_side: 1,
        engine_time: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    }
}

fn update(order_id: Uuid, status: i16, filled_base: u128, filled_quote: u128) -> OrderUpdate {
    OrderUpdate {
        order_id,
        status,
        cancel_reason: None,
        filled_base,
        filled_quote,
        limit_price: None,
    }
}

/// Returns `(status, filled_base, filled_quote, limit_price)`.
async fn order_state(pool: &PgPool, id: Uuid) -> (Option<i16>, String, String, Option<String>) {
    sqlx::query_as(
        "SELECT status, filled_base_quantity::text, filled_quote_quantity::text, limit_price::text \
         FROM orders WHERE order_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read order")
}

async fn two_orders(pool: &PgPool) -> (World, Uuid, Uuid) {
    let w = world(pool).await;
    let (b, s) = (party(pool, &w).await, party(pool, &w).await);
    let buy = order(pool, &w, &b, true).await;
    let sell = order(pool, &w, &s, false).await;
    (w, buy, sell)
}

// --- store ------------------------------------------------------------------

#[tokio::test]
async fn store_known_trade_ids_returns_only_existing() {
    let pool = pool().await;
    let (w, buy, sell) = two_orders(&pool).await;
    let t = trade(&w, buy, sell);
    store::commit(&pool, std::slice::from_ref(&t), &[])
        .await
        .unwrap();
    let got = store::known_trade_ids(&pool, &[t.trade_id, Uuid::new_v4()])
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert!(got.contains(&t.trade_id));
}

#[tokio::test]
async fn store_commit_inserts_a_trade_once() {
    let pool = pool().await;
    let (w, buy, sell) = two_orders(&pool).await;
    let t = trade(&w, buy, sell);
    let first = store::commit(&pool, std::slice::from_ref(&t), &[])
        .await
        .unwrap();
    let second = store::commit(&pool, &[t], &[]).await.unwrap();
    assert_eq!(first.trades_inserted, 1);
    assert_eq!(second.trades_inserted, 0);
}

#[tokio::test]
async fn store_first_report_applies_when_status_is_null() {
    let pool = pool().await;
    let (_, buy, _) = two_orders(&pool).await;
    assert_eq!(order_state(&pool, buy).await.0, None);
    let stats = store::commit(&pool, &[], &[update(buy, 1, 0, 0)])
        .await
        .unwrap();
    assert_eq!(stats.orders_updated, 1);
    assert_eq!(order_state(&pool, buy).await.0, Some(1));
}

#[tokio::test]
async fn store_update_is_ignored_when_the_order_is_already_terminal() {
    let pool = pool().await;
    let (_, buy, _) = two_orders(&pool).await;
    store::commit(&pool, &[], &[update(buy, 3, 10, 10_000)])
        .await
        .unwrap();
    let stats = store::commit(&pool, &[], &[update(buy, 2, 10, 10_000)])
        .await
        .unwrap();
    assert_eq!((stats.orders_updated, stats.orders_skipped), (0, 1));
    assert_eq!(order_state(&pool, buy).await.0, Some(3));
}

#[tokio::test]
async fn store_update_is_ignored_when_filled_would_go_down() {
    let pool = pool().await;
    let (_, buy, _) = two_orders(&pool).await;
    store::commit(&pool, &[], &[update(buy, 2, 5, 5000)])
        .await
        .unwrap();
    let stats = store::commit(&pool, &[], &[update(buy, 2, 3, 3000)])
        .await
        .unwrap();
    assert_eq!(stats.orders_skipped, 1);
    let state = order_state(&pool, buy).await;
    assert_eq!((state.1.as_str(), state.2.as_str()), ("5", "5000"));
}

#[tokio::test]
async fn store_update_applies_when_filled_is_equal() {
    let pool = pool().await;
    let (_, buy, _) = two_orders(&pool).await;
    store::commit(&pool, &[], &[update(buy, 1, 0, 0)])
        .await
        .unwrap();
    let mut replaced = update(buy, 1, 0, 0);
    replaced.limit_price = Some(1050);
    let stats = store::commit(&pool, &[], &[replaced]).await.unwrap();
    assert_eq!(stats.orders_updated, 1);
    assert_eq!(order_state(&pool, buy).await.3.as_deref(), Some("1050"));
}

#[tokio::test]
async fn store_commit_is_one_transaction() {
    let pool = pool().await;
    let (w, buy, sell) = two_orders(&pool).await;
    let t = trade(&w, buy, sell);
    // status 9 violates the CHECK on orders.status, so the whole call must roll back
    let result = store::commit(&pool, std::slice::from_ref(&t), &[update(buy, 9, 0, 0)]).await;
    assert!(result.is_err());
    assert!(
        store::known_trade_ids(&pool, &[t.trade_id])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn store_big_values_round_trip() {
    let pool = pool().await;
    let (_, buy, _) = two_orders(&pool).await;
    let big = 10u128.pow(38);
    store::commit(&pool, &[], &[update(buy, 2, big, big)])
        .await
        .unwrap();
    let state = order_state(&pool, buy).await;
    assert_eq!(state.1, big.to_string());
    assert_eq!(state.2, big.to_string());
}

#[tokio::test]
async fn store_load_orders_and_markets_return_what_the_plan_needs() {
    let pool = pool().await;
    let (w, buy, sell) = two_orders(&pool).await;
    let orders = store::load_orders(&pool, &[buy, sell, Uuid::new_v4()])
        .await
        .unwrap();
    assert_eq!(orders.len(), 2);
    assert!(orders[&buy].buyer && !orders[&sell].buyer);
    assert_eq!(orders[&buy].limit_price, Some(1000));
    assert_eq!(orders[&buy].market_id, w.market_id);
    let markets = store::load_markets(&pool, &[w.symbol.clone(), "NOPE".to_string()])
        .await
        .unwrap();
    assert_eq!(markets.len(), 1);
    assert_eq!(markets[&w.symbol].base_ledger_id, w.base_ledger);
    assert_eq!(markets[&w.symbol].quote_ledger_id, w.quote_ledger);
    assert_eq!(markets[&w.symbol].fee_account_id, w.fee_account);
}

// --- ledger (tigerbeetle) -----------------------------------------------------

fn tb_client() -> tb::Client {
    let cluster = std::env::var("TB_CLUSTER_ID")
        .map(|v| v.parse().unwrap())
        .unwrap_or(0);
    let addresses =
        std::env::var("TB_IP_ADRESSES").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    tb::Client::new(cluster, &addresses).expect("tigerbeetle client")
}

/// Ledger numbers are cluster-wide, so every test takes a fresh one.
fn fresh_tb_ledger() -> u32 {
    (Uuid::new_v4().as_u128() as u32) | 1
}

async fn tb_account(c: &tb::Client, ledger: u32, flags: tb::AccountFlags) -> u128 {
    tb_account_with_id(c, tb::id(), ledger, flags).await
}

async fn tb_account_with_id(
    c: &tb::Client,
    id: u128,
    ledger: u32,
    flags: tb::AccountFlags,
) -> u128 {
    let r = c
        .create_accounts(&[tb::Account {
            id,
            ledger,
            code: 1,
            flags,
            ..Default::default()
        }])
        .await
        .expect("create account");
    assert!(
        r.is_empty(),
        "account rejected: {:?}",
        r.first().map(|x| &x.result)
    );
    id
}

fn transfer(id: u128, debit: u128, credit: u128, amount: u128, ledger: u32) -> tb::Transfer {
    tb::Transfer {
        id,
        debit_account_id: debit,
        credit_account_id: credit,
        amount,
        ledger,
        code: TransferCodeType::Trade as u16,
        ..Default::default()
    }
}

async fn balance(c: &tb::Client, id: u128) -> (u128, u128) {
    let a = c.lookup_accounts(&[id]).await.expect("lookup").remove(0);
    (a.debits_posted, a.credits_posted)
}

const MUST_NOT_OVERDRAW: tb::AccountFlags = tb::AccountFlags::DebitsMustNotExceedCredits;

#[tokio::test]
async fn ledger_sending_the_same_chain_twice_succeeds() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let source = tb_account(&c, l, tb::AccountFlags::empty()).await;
    let user = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let mut first = transfer(tb::id(), source, user, 100, l);
    first.flags = tb::TransferFlags::Linked;
    let chain = [first, transfer(tb::id(), source, user, 50, l)];
    ledger::send(&c, &chain).await.unwrap();
    ledger::send(&c, &chain).await.unwrap(); // Exists is success
    assert_eq!(balance(&c, user).await, (0, 150));
}

#[tokio::test]
async fn ledger_chain_without_funds_is_rejected() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let source = tb_account(&c, l, tb::AccountFlags::empty()).await;
    let poor = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let err = ledger::send(&c, &[transfer(tb::id(), poor, source, 10, l)])
        .await
        .unwrap_err();
    assert!(
        matches!(err, LedgerError::Rejected { index: 0, .. }),
        "{err:?}"
    );
}

#[tokio::test]
async fn ledger_empty_batch_is_a_noop() {
    ledger::send(&tb_client(), &[]).await.unwrap();
}

fn refund(id: u128, hold: u128, user: u128, l: u32) -> tb::Transfer {
    let mut t = transfer(id, hold, user, u128::MAX, l);
    t.code = TransferCodeType::Refund as u16;
    t.flags = tb::TransferFlags::BalancingDebit;
    t
}

#[tokio::test]
async fn ledger_balancing_refund_moves_the_remainder_and_is_repeatable() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let source = tb_account(&c, l, tb::AccountFlags::empty()).await;
    let hold = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let user = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let counterparty = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    ledger::send(&c, &[transfer(tb::id(), source, hold, 100, l)])
        .await
        .unwrap();
    ledger::send(&c, &[transfer(tb::id(), hold, counterparty, 30, l)])
        .await
        .unwrap();

    let r = refund(tb::id(), hold, user, l);
    ledger::send(&c, &[r]).await.unwrap();
    assert_eq!(balance(&c, user).await, (0, 70));
    assert_eq!(balance(&c, hold).await, (100, 100));

    // the replay of the same refund: same id, same fields
    ledger::send(&c, &[r]).await.unwrap();
    assert_eq!(balance(&c, user).await, (0, 70));
}

#[tokio::test]
async fn ledger_refund_of_an_empty_hold_account_is_ok() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let hold = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let user = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let r = refund(tb::id(), hold, user, l);
    ledger::send(&c, &[r]).await.unwrap();
    ledger::send(&c, &[r]).await.unwrap();
    assert_eq!(balance(&c, user).await, (0, 0));
}

#[tokio::test]
async fn ledger_replayed_chain_does_not_block_a_new_chain_in_the_same_batch() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let source = tb_account(&c, l, tb::AccountFlags::empty()).await;
    let user = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let linked = |mut t: tb::Transfer| {
        t.flags = tb::TransferFlags::Linked;
        t
    };
    let old = [
        linked(transfer(tb::id(), source, user, 10, l)),
        transfer(tb::id(), source, user, 5, l),
    ];
    let new = [
        linked(transfer(tb::id(), source, user, 20, l)),
        transfer(tb::id(), source, user, 7, l),
    ];
    ledger::send(&c, &old).await.unwrap();
    ledger::send(&c, &[old[0], old[1], new[0], new[1]])
        .await
        .unwrap();
    assert_eq!(balance(&c, user).await, (0, 10 + 5 + 20 + 7));
}

#[tokio::test]
async fn ledger_failed_leg_is_named_not_its_chain_mates() {
    let c = tb_client();
    let l = fresh_tb_ledger();
    let source = tb_account(&c, l, tb::AccountFlags::empty()).await;
    let user = tb_account(&c, l, MUST_NOT_OVERDRAW).await;
    let mut ok_leg = transfer(tb::id(), source, user, 10, l);
    ok_leg.flags = tb::TransferFlags::Linked;
    let overdraw = transfer(tb::id(), user, source, 1_000, l); // user only has 10, and only if leg 1 lands
    let err = ledger::send(&c, &[ok_leg, overdraw]).await.unwrap_err();
    match err {
        LedgerError::Rejected { index, result } => {
            assert_eq!(index, 1);
            assert_ne!(result, tb::CreateTransferResult::LinkedEventFailed);
        }
        e => panic!("{e:?}"),
    }
    assert_eq!(balance(&c, user).await, (0, 0)); // the chain rolled back
}

// --- single consumer lock -----------------------------------------------------

fn fresh_lock_key() -> i64 {
    Uuid::new_v4().as_u128() as i64
}

#[tokio::test]
async fn lock_second_holder_is_refused() {
    let key = fresh_lock_key();
    let first = lock::try_lock(&database_url(), key).await.unwrap();
    let second = lock::try_lock(&database_url(), key).await;
    assert!(second.is_err());
    first.release().await;
}

#[tokio::test]
async fn lock_can_be_taken_again_after_release() {
    let key = fresh_lock_key();
    lock::try_lock(&database_url(), key)
        .await
        .unwrap()
        .release()
        .await;
    lock::try_lock(&database_url(), key)
        .await
        .unwrap()
        .release()
        .await;
}

#[tokio::test]
async fn lock_is_released_when_the_guard_is_dropped() {
    let key = fresh_lock_key();
    drop(lock::try_lock(&database_url(), key).await.unwrap());
    // postgres notices the closed connection a moment later
    for _ in 0..25 {
        if let Ok(guard) = lock::try_lock(&database_url(), key).await {
            guard.release().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    panic!("the lock was still held 5s after the guard was dropped");
}

// --- end to end: a fake router, the real consumer, real Postgres and TigerBeetle -----------------

use proto::trading_engine_server::{TradingEngine, TradingEngineServer};
use proto::{
    EventItem, ExecutionReport, Id, OrderStatus, Side, TradeSettlement, Uint128, event_item::Body,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tonic::codegen::tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Request, Response, Status};

#[derive(Clone, Copy)]
enum Ending {
    Hang,  // stay open and idle
    Close, // end the stream cleanly
    Error, // break the stream
}

struct Sub {
    items: Vec<EventItem>,
    ending: Ending,
}

struct FakeRouter {
    subs: Mutex<VecDeque<Sub>>,
    subscribed: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl TradingEngine for FakeRouter {
    async fn set_market(
        &self,
        _: Request<proto::SetMarketRequest>,
    ) -> Result<Response<proto::SetMarketResponse>, Status> {
        Err(Status::unimplemented("fake"))
    }
    async fn place_order(
        &self,
        _: Request<proto::PlaceOrderRequest>,
    ) -> Result<Response<proto::PlaceOrderResponse>, Status> {
        Err(Status::unimplemented("fake"))
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

    type SubscribeEventsStream = ReceiverStream<Result<EventItem, Status>>;
    async fn subscribe_events(
        &self,
        _: Request<proto::SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        self.subscribed.fetch_add(1, Ordering::SeqCst);
        let sub = self.subs.lock().unwrap().pop_front();
        let (tx, rx) = mpsc::channel(1024);
        tokio::spawn(async move {
            if let Some(sub) = sub {
                for item in sub.items {
                    if tx.send(Ok(item)).await.is_err() {
                        return;
                    }
                }
                match sub.ending {
                    Ending::Close => return,
                    Ending::Error => {
                        let _ = tx
                            .send(Err(Status::unavailable("router lost the engine")))
                            .await;
                        return;
                    }
                    Ending::Hang => {}
                }
            }
            tx.closed().await; // idle until the consumer goes away
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type SubscribeMarketDataStream = ReceiverStream<Result<proto::MarketDataItem, Status>>;
    async fn subscribe_market_data(
        &self,
        _: Request<proto::SubscribeMarketDataRequest>,
    ) -> Result<Response<Self::SubscribeMarketDataStream>, Status> {
        Err(Status::unimplemented("fake"))
    }
}

/// Serves the scripted subscriptions (the n-th subscribe gets the n-th script) and returns its
/// URL.
async fn serve(subs: Vec<Sub>) -> (String, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let subscribed = Arc::new(AtomicUsize::new(0));
    let router = FakeRouter {
        subs: Mutex::new(subs.into()),
        subscribed: subscribed.clone(),
    };
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(TradingEngineServer::new(router))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    (url, subscribed)
}

fn config(router_url: String, lock_key: i64) -> Config {
    Config {
        engine_url: router_url,
        database_url: database_url(),
        tb_cluster_id: std::env::var("TB_CLUSTER_ID")
            .map(|v| v.parse().unwrap())
            .unwrap_or(0),
        tb_addresses: std::env::var("TB_IP_ADRESSES")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
        batch_max_items: 100,
        batch_max_wait: Duration::from_millis(20),
        queue_depth: 2,
        lock_key,
    }
}

type Running = (JoinHandle<Result<(), ConsumerError>>, CancellationToken);

fn start(cfg: Config) -> Running {
    let cancel = CancellationToken::new();
    (tokio::spawn(consumer::run(cfg, cancel.clone())), cancel)
}

async fn stop(running: Running) {
    let (task, cancel) = running;
    cancel.cancel();
    let result = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("the consumer did not stop")
        .unwrap();
    assert!(result.is_ok(), "{result:?}");
}

async fn wait_for<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for: {what}");
}

fn ev(w: &World, body: Body) -> EventItem {
    EventItem {
        sequence: 1,
        engine_time: Some(proto::Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        }),
        market_symbol: w.symbol.clone(),
        body: Some(body),
    }
}

fn id128(u: Uuid) -> Option<Id> {
    Some(Id::from(u.as_u128()))
}

fn qty(n: u128) -> Option<Uint128> {
    Some(Uint128::from(n))
}

struct Scenario {
    w: World,
    buyer: Party,
    seller: Party,
    buy_order: Uuid,
    sell_order: Uuid,
    /// Each scenario has its own trade id. A trade that is already recorded is skipped, so a shared
    /// id would make parallel tests (and reruns on the same database) skip each other's trades.
    trade: u128,
}

const BASE_QTY: u128 = 1000;
const QUOTE_QTY: u128 = 1_000_000;
const BUYER_FEE: u128 = 3000;
const SELLER_FEE: u128 = 1000;
const BUY_RESERVATION: u128 = 1_010_000;

/// Postgres rows and funded TigerBeetle accounts for one buyer and one seller.
///
/// Each order has a hold account (id = order id) holding what it reserved: quote for the buyer,
/// base for the seller.
async fn scenario(pool: &PgPool, c: &tb::Client) -> Scenario {
    let w = world(pool).await;
    let (buyer, seller) = (party(pool, &w).await, party(pool, &w).await);
    let buy_order = order(pool, &w, &buyer, true).await;
    let sell_order = order(pool, &w, &seller, false).await;
    let (base_l, quote_l) = (w.base_ledger as u32, w.quote_ledger as u32);

    for (id, ledger) in [
        (buyer.base, base_l),
        (buyer.quote, quote_l),
        (seller.base, base_l),
        (seller.quote, quote_l),
        (w.fee_account, quote_l),
        (buy_order, quote_l),
        (sell_order, base_l),
    ] {
        tb_account_with_id(c, id.as_u128(), ledger, MUST_NOT_OVERDRAW).await;
    }
    let source_base = tb_account(c, base_l, tb::AccountFlags::empty()).await;
    let source_quote = tb_account(c, quote_l, tb::AccountFlags::empty()).await;
    ledger::send(
        c,
        &[
            transfer(
                tb::id(),
                source_quote,
                buy_order.as_u128(),
                BUY_RESERVATION,
                quote_l,
            ),
            transfer(
                tb::id(),
                source_base,
                sell_order.as_u128(),
                BASE_QTY,
                base_l,
            ),
        ],
    )
    .await
    .unwrap();
    Scenario {
        w,
        buyer,
        seller,
        buy_order,
        sell_order,
        trade: tb::id(),
    }
}

fn trade_event(s: &Scenario, trade: u128, price: u128) -> EventItem {
    ev(
        &s.w,
        Body::TradeSettlement(TradeSettlement {
            trade_id: Some(Id::from(trade)),
            price: qty(price),
            base_quantity: qty(BASE_QTY),
            quote_quantity: qty(QUOTE_QTY),
            taker_side: Side::Buy as i32,
            buyer_order_id: id128(s.buy_order),
            buyer_account_holder: id128(s.buyer.user),
            buyer_base_account: id128(s.buyer.base),
            buyer_quote_account: id128(s.buyer.quote),
            buyer_fee: qty(BUYER_FEE),
            seller_order_id: id128(s.sell_order),
            seller_account_holder: id128(s.seller.user),
            seller_base_account: id128(s.seller.base),
            seller_quote_account: id128(s.seller.quote),
            seller_fee: qty(SELLER_FEE),
            fee_account: id128(s.w.fee_account),
        }),
    )
}

fn report_event(
    w: &World,
    order: Uuid,
    status: OrderStatus,
    filled_base: u128,
    filled_quote: u128,
) -> EventItem {
    ev(
        w,
        Body::ExecutionReport(ExecutionReport {
            order_id: id128(order),
            status: status as i32,
            filled_base_quantity: qty(filled_base),
            filled_quote_quantity: qty(filled_quote),
            limit_price: qty(1000),
            ..Default::default()
        }),
    )
}

fn full_history(s: &Scenario, trade: u128) -> Vec<EventItem> {
    vec![
        trade_event(s, trade, 1000),
        report_event(&s.w, s.buy_order, OrderStatus::Filled, BASE_QTY, QUOTE_QTY),
        report_event(&s.w, s.sell_order, OrderStatus::Filled, BASE_QTY, QUOTE_QTY),
    ]
}

async fn is_settled(pool: &PgPool, s: &Scenario) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM orders WHERE order_id = ANY($1) AND status = 3",
    )
    .bind(vec![s.buy_order, s.sell_order])
    .fetch_one(pool)
    .await
    .unwrap()
        == 2
}

/// The money must have moved exactly once, however many times the events were delivered.
async fn assert_settled_exactly_once(pool: &PgPool, c: &tb::Client, s: &Scenario) {
    let id = |u: Uuid| u.as_u128();
    assert_eq!(
        balance(c, id(s.buyer.base)).await,
        (0, BASE_QTY),
        "buyer receives the base"
    );
    assert_eq!(
        balance(c, id(s.seller.quote)).await,
        (SELLER_FEE, QUOTE_QTY),
        "seller receives the quote, pays its fee"
    );
    assert_eq!(
        balance(c, id(s.w.fee_account)).await,
        (0, BUYER_FEE + SELLER_FEE)
    );
    // the buyer's hold: spent quote + buyer fee, the rest refunded
    assert_eq!(
        balance(c, id(s.buy_order)).await,
        (BUY_RESERVATION, BUY_RESERVATION)
    );
    assert_eq!(
        balance(c, id(s.buyer.quote)).await,
        (0, BUY_RESERVATION - QUOTE_QTY - BUYER_FEE)
    );
    assert_eq!(balance(c, id(s.sell_order)).await, (BASE_QTY, BASE_QTY));
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM trades WHERE market_id = $1")
        .bind(s.w.market_id)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(trades, 1);
    let filled = order_state(pool, s.buy_order).await;
    assert_eq!(
        (filled.0, filled.1.as_str(), filled.2.as_str()),
        (Some(3), "1000", "1000000")
    );
}

#[tokio::test]
async fn e2e_trade_settles_and_orders_update() {
    let (pool, c) = (pool().await, tb_client());
    let s = scenario(&pool, &c).await;
    assert_eq!(
        order_state(&pool, s.buy_order).await.0,
        None,
        "a new order has no status yet"
    );
    let (url, _) = serve(vec![Sub {
        items: full_history(&s, s.trade),
        ending: Ending::Hang,
    }])
    .await;
    let running = start(config(url, fresh_lock_key()));
    wait_for("both orders FILLED", || is_settled(&pool, &s)).await;
    assert_settled_exactly_once(&pool, &c, &s).await;
    stop(running).await;
}

#[tokio::test]
async fn e2e_replaying_the_same_events_changes_nothing() {
    let (pool, c) = (pool().await, tb_client());
    let s = scenario(&pool, &c).await;
    // an order that only the second delivery mentions, last: once it is updated, everything
    // before it in the replay has been processed
    let marker = order(&pool, &s.w, &s.buyer, true).await;
    let mut replay = full_history(&s, s.trade);
    replay.push(report_event(&s.w, marker, OrderStatus::New, 0, 0));
    let (url, subscribed) = serve(vec![
        Sub {
            items: full_history(&s, s.trade),
            ending: Ending::Close,
        },
        Sub {
            items: replay,
            ending: Ending::Hang,
        },
    ])
    .await;
    let running = start(config(url, fresh_lock_key()));
    wait_for("the replay reached the marker order", || async {
        order_state(&pool, marker).await.0 == Some(1)
    })
    .await;
    assert!(subscribed.load(Ordering::SeqCst) >= 2);
    assert_settled_exactly_once(&pool, &c, &s).await;
    stop(running).await;
}

#[tokio::test]
async fn e2e_stream_drop_mid_history_then_full_replay_ends_consistent() {
    let (pool, c) = (pool().await, tb_client());
    let s = scenario(&pool, &c).await;
    let history = full_history(&s, s.trade);
    let (url, _) = serve(vec![
        Sub {
            items: history[..2].to_vec(),
            ending: Ending::Error,
        },
        Sub {
            items: history,
            ending: Ending::Hang,
        },
    ])
    .await;
    let running = start(config(url, fresh_lock_key()));
    wait_for("both orders FILLED", || is_settled(&pool, &s)).await;
    assert_settled_exactly_once(&pool, &c, &s).await;
    stop(running).await;
}

async fn finishes_with_error(running: Running) -> String {
    let (task, _cancel) = running;
    let result = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("the consumer should have stopped by itself")
        .unwrap();
    result
        .expect_err("the consumer should have failed")
        .to_string()
}

#[tokio::test]
async fn e2e_report_for_an_order_missing_from_postgres_stops_the_consumer() {
    let pool = pool().await;
    let w = world(&pool).await;
    let (url, _) = serve(vec![Sub {
        items: vec![report_event(&w, Uuid::new_v4(), OrderStatus::New, 0, 0)],
        ending: Ending::Hang,
    }])
    .await;
    let message = finishes_with_error(start(config(url, fresh_lock_key()))).await;
    assert!(message.contains("not in postgres"), "{message}");
}

#[tokio::test]
async fn e2e_invalid_trade_stops_the_consumer_and_moves_nothing() {
    let (pool, c) = (pool().await, tb_client());
    let s = scenario(&pool, &c).await;
    // above the buyer's limit price of 1000
    let (url, _) = serve(vec![Sub {
        items: vec![trade_event(&s, s.trade, 5000)],
        ending: Ending::Hang,
    }])
    .await;
    let message = finishes_with_error(start(config(url, fresh_lock_key()))).await;
    assert!(message.contains("validation"), "{message}");
    assert_eq!(balance(&c, s.buyer.base.as_u128()).await, (0, 0));
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM trades WHERE market_id = $1")
        .bind(s.w.market_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(trades, 0);
}

#[tokio::test]
async fn e2e_second_consumer_cannot_start_while_the_first_runs() {
    let key = fresh_lock_key();
    let (url, subscribed) = serve(vec![]).await;
    let first = start(config(url.clone(), key));
    wait_for("the first consumer subscribed", || async {
        subscribed.load(Ordering::SeqCst) >= 1
    })
    .await;
    let message = finishes_with_error(start(config(url, key))).await;
    assert!(message.contains("advisory lock"), "{message}");
    stop(first).await;
}

fn report_with_limit(w: &World, order: Uuid, limit: u128) -> EventItem {
    let mut event = report_event(w, order, OrderStatus::New, 0, 0);
    if let Some(Body::ExecutionReport(report)) = event.body.as_mut() {
        report.limit_price = qty(limit);
    }
    event
}

#[tokio::test]
async fn e2e_a_repricing_and_the_trade_after_it_in_different_batches() {
    let (pool, c) = (pool().await, tb_client());
    let s = scenario(&pool, &c).await; // both orders have limit 1000 in postgres
    // the buyer raises the limit to 1300 and matches at 1200. With one event per batch, the trade
    // is prepared while the batch with the repricing is not saved yet, so postgres still says 1000
    let history = vec![
        report_with_limit(&s.w, s.buy_order, 1300),
        trade_event(&s, s.trade, 1200),
        report_event(&s.w, s.buy_order, OrderStatus::Filled, BASE_QTY, QUOTE_QTY),
        report_event(&s.w, s.sell_order, OrderStatus::Filled, BASE_QTY, QUOTE_QTY),
    ];
    let (url, _) = serve(vec![Sub {
        items: history,
        ending: Ending::Hang,
    }])
    .await;
    let mut cfg = config(url, fresh_lock_key());
    cfg.batch_max_items = 1;
    let running = start(cfg);
    wait_for("both orders FILLED", || is_settled(&pool, &s)).await;
    assert_settled_exactly_once(&pool, &c, &s).await;
    stop(running).await;
}
