//! End-to-end tests that drive the real HTTP API against a real Postgres and a
//! real (single-replica, development-mode) TigerBeetle.
//!
//! The suite expects the backend to already be listening. Use
//! `scripts/integration-test.sh` (also what CI runs) to bring up Postgres +
//! TigerBeetle + the backend and then run `cargo test --test integration_test`.
//! Point the tests at a different host with `BACKEND_URL`.

use cn_tigerbeetle as tb;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

// --- helpers ----------------------------------------------------------------

fn base() -> String {
    std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8000".to_string()) + "/api/v1"
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("build client")
}

/// A value unlikely to collide across parallel tests or repeated CI runs.
fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

/// A fresh ledger symbol (`TEXT UNIQUE` in Postgres, so it must differ per run).
fn unique_symbol() -> String {
    format!("T{}", &Uuid::new_v4().simple().to_string()[..10]).to_uppercase()
}

/// Asserts the status code, then returns the parsed body (or `Value::Null` when
/// the response has no body). On mismatch the panic message carries the body,
/// which is where the handlers put their diagnostics.
async fn expect_status(res: reqwest::Response, want: u16) -> Value {
    let got = res.status().as_u16();
    let text = res.text().await.unwrap_or_default();
    assert_eq!(got, want, "expected {want}, got {got}; body: {text}");
    if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or(Value::Null)
    }
}

async fn create_user(c: &reqwest::Client) -> String {
    let name = unique("user");
    let body = expect_status(
        c.post(format!("{}/users", base()))
            .json(&json!({ "name": name }))
            .send()
            .await
            .expect("send create user"),
        201,
    )
    .await;
    body["user_id"].as_str().expect("user_id in body").to_string()
}

/// Creates a ledger and returns its symbol.
async fn create_ledger(c: &reqwest::Client, decimals: i64) -> String {
    let symbol = unique_symbol();
    expect_status(
        c.post(format!("{}/ledgers", base()))
            .json(&json!({ "symbol": symbol, "name": unique("ledger"), "decimals": decimals }))
            .send()
            .await
            .expect("send create ledger"),
        201,
    )
    .await;
    symbol
}

async fn set_ledger_enabled(c: &reqwest::Client, symbol: &str, enabled: bool) {
    expect_status(
        c.patch(format!("{}/ledgers/{symbol}", base()))
            .json(&json!({ "enabled": enabled }))
            .send()
            .await
            .expect("send patch ledger"),
        200,
    )
    .await;
}

/// POSTs an account and returns `(status, body)`. The handler answers 201 when
/// it manages to create the TigerBeetle account inline (within its 500ms
/// budget) and 202 when it defers to the relay, so callers must accept both.
async fn create_account(
    c: &reqwest::Client,
    user_id: &str,
    ledger_symbol: &str,
) -> (u16, Value) {
    let res = c
        .post(format!("{}/users/{user_id}/accounts", base()))
        .json(&json!({
            "name": unique("account"),
            "ledger_symbol": ledger_symbol,
        }))
        .send()
        .await
        .expect("send create account");
    let status = res.status().as_u16();
    let body: Value = res.json().await.unwrap_or(Value::Null);
    (status, body)
}

/// Polls `GET /accounts/{id}` until the outbox relay (or the inline path) has
/// flipped the account to `active`. The relay only picks rows up after a 5s
/// grace period, so this can legitimately take several seconds.
async fn wait_for_active(c: &reqwest::Client, account_id: &str) -> Value {
    for _ in 0..60 {
        let res = c
            .get(format!("{}/accounts/{account_id}", base()))
            .send()
            .await
            .expect("send fetch account");
        if res.status().as_u16() == 200 {
            let body: Value = res.json().await.expect("account json");
            if body["account_status"] == "active" {
                return body;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    panic!("account {account_id} never became active");
}

// --- users ----------------------------------------------------------------

#[tokio::test]
async fn create_user_returns_201_with_body() {
    let c = client();
    let name = unique("ada");
    let body = expect_status(
        c.post(format!("{}/users", base()))
            .json(&json!({ "name": name }))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;

    assert!(Uuid::parse_str(body["user_id"].as_str().unwrap()).is_ok());
    assert_eq!(body["user_name"], name);
}

#[tokio::test]
async fn fetch_user_returns_created_user() {
    let c = client();
    let id = create_user(&c).await;

    let body = expect_status(
        c.get(format!("{}/users/{id}", base())).send().await.unwrap(),
        200,
    )
    .await;
    assert_eq!(body["user_id"], id);
}

#[tokio::test]
async fn list_users_includes_created_user() {
    let c = client();
    let id = create_user(&c).await;

    let body = expect_status(
        c.get(format!("{}/users", base())).send().await.unwrap(),
        200,
    )
    .await;
    let found = body
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u["user_id"] == id);
    assert!(found, "created user {id} missing from list");
}

#[tokio::test]
async fn fetch_unknown_user_returns_404() {
    let c = client();
    let res = c
        .get(format!("{}/users/{}", base(), Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    expect_status(res, 404).await;
}

#[tokio::test]
async fn fetch_user_with_malformed_uuid_returns_400() {
    let c = client();
    let res = c
        .get(format!("{}/users/not-a-uuid", base()))
        .send()
        .await
        .unwrap();
    expect_status(res, 400).await;
}

#[tokio::test]
async fn create_user_without_name_returns_422() {
    let c = client();
    let res = c
        .post(format!("{}/users", base()))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    expect_status(res, 422).await;
}

// --- ledgers --------------------------------------------------------------

#[tokio::test]
async fn list_ledgers_includes_seeded_ledgers() {
    let c = client();
    let body = expect_status(
        c.get(format!("{}/ledgers", base())).send().await.unwrap(),
        200,
    )
    .await;
    let ledgers = body.as_array().unwrap();

    let usd = ledgers.iter().find(|l| l["symbol"] == "USD").expect("USD ledger");
    assert_eq!(usd["decimals"], 2);
    assert_eq!(usd["enabled"], true);
    assert!(ledgers.iter().any(|l| l["symbol"] == "EUR"));
    let btc = ledgers.iter().find(|l| l["symbol"] == "BTC").expect("BTC ledger");
    assert_eq!(btc["decimals"], 8);
}

#[tokio::test]
async fn fetch_ledger_by_symbol_returns_it() {
    let c = client();
    let body = expect_status(
        c.get(format!("{}/ledgers/USD", base())).send().await.unwrap(),
        200,
    )
    .await;
    assert_eq!(body["symbol"], "USD");
    assert_eq!(body["decimals"], 2);
}

#[tokio::test]
async fn fetch_unknown_ledger_returns_404() {
    let c = client();
    let res = c
        .get(format!("{}/ledgers/{}", base(), unique_symbol()))
        .send()
        .await
        .unwrap();
    expect_status(res, 404).await;
}

#[tokio::test]
async fn create_ledger_returns_201_and_is_visible() {
    let c = client();
    let symbol = unique_symbol();
    let name = unique("ledger");

    let created = expect_status(
        c.post(format!("{}/ledgers", base()))
            .json(&json!({ "symbol": symbol, "name": name, "decimals": 4 }))
            .send()
            .await
            .unwrap(),
        201,
    )
    .await;
    assert_eq!(created["symbol"], symbol);
    assert_eq!(created["name"], name);
    assert_eq!(created["decimals"], 4);
    assert_eq!(created["enabled"], true);
    assert!(created["ledger_id"].is_number());

    // it should now be served from the in-memory cache too
    let fetched = expect_status(
        c.get(format!("{}/ledgers/{symbol}", base())).send().await.unwrap(),
        200,
    )
    .await;
    assert_eq!(fetched["ledger_id"], created["ledger_id"]);
}

#[tokio::test]
async fn create_ledger_with_duplicate_symbol_returns_409() {
    let c = client();
    let symbol = create_ledger(&c, 2).await;

    let res = c
        .post(format!("{}/ledgers", base()))
        .json(&json!({ "symbol": symbol, "name": unique("dup"), "decimals": 2 }))
        .send()
        .await
        .unwrap();
    expect_status(res, 409).await;
}

#[tokio::test]
async fn create_ledger_with_decimals_out_of_range_returns_400() {
    let c = client();
    let res = c
        .post(format!("{}/ledgers", base()))
        .json(&json!({ "symbol": unique_symbol(), "name": unique("bad"), "decimals": 25 }))
        .send()
        .await
        .unwrap();
    expect_status(res, 400).await;
}

#[tokio::test]
async fn create_ledger_missing_field_returns_422() {
    let c = client();
    let res = c
        .post(format!("{}/ledgers", base()))
        .json(&json!({ "symbol": unique_symbol() }))
        .send()
        .await
        .unwrap();
    expect_status(res, 422).await;
}

#[tokio::test]
async fn set_ledger_enabled_toggles_flag() {
    let c = client();
    let symbol = create_ledger(&c, 2).await;

    let disabled = expect_status(
        c.patch(format!("{}/ledgers/{symbol}", base()))
            .json(&json!({ "enabled": false }))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(disabled["enabled"], false);

    let fetched = expect_status(
        c.get(format!("{}/ledgers/{symbol}", base())).send().await.unwrap(),
        200,
    )
    .await;
    assert_eq!(fetched["enabled"], false);

    let reenabled = expect_status(
        c.patch(format!("{}/ledgers/{symbol}", base()))
            .json(&json!({ "enabled": true }))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(reenabled["enabled"], true);
}

#[tokio::test]
async fn set_enabled_on_unknown_ledger_returns_404() {
    let c = client();
    let res = c
        .patch(format!("{}/ledgers/{}", base(), unique_symbol()))
        .json(&json!({ "enabled": false }))
        .send()
        .await
        .unwrap();
    expect_status(res, 404).await;
}

// --- accounts (exercise the Postgres -> outbox -> relay -> TigerBeetle path) --

#[tokio::test]
async fn create_account_eventually_activates() {
    let c = client();
    let user_id = create_user(&c).await;
    let symbol = create_ledger(&c, 2).await;

    let (status, body) = create_account(&c, &user_id, &symbol).await;
    assert!(
        status == 201 || status == 202,
        "unexpected status {status}; body {body}"
    );
    assert_eq!(body["account_user_id"], user_id);
    assert_eq!(body["account_code_type"], 1);
    assert!(body["account_ledger_id"].is_number());
    let account_id = body["account_id"].as_str().expect("account_id").to_string();

    let active = wait_for_active(&c, &account_id).await;
    assert_eq!(active["account_status"], "active");
    assert_eq!(active["account_id"], account_id);
}

#[tokio::test]
async fn create_account_with_unknown_ledger_returns_404() {
    let c = client();
    let user_id = create_user(&c).await;
    let (status, _) = create_account(&c, &user_id, &unique_symbol()).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn create_account_on_disabled_ledger_returns_400() {
    let c = client();
    let user_id = create_user(&c).await;
    let symbol = create_ledger(&c, 2).await;
    set_ledger_enabled(&c, &symbol, false).await;

    let (status, _) = create_account(&c, &user_id, &symbol).await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn create_account_for_unknown_user_returns_400() {
    let c = client();
    // USD is seeded and enabled; the failure must come from the user FK.
    let (status, _) = create_account(&c, &Uuid::new_v4().to_string(), "USD").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn create_account_ignores_client_code_type() {
    let c = client();
    let user_id = create_user(&c).await;
    let res = c
        .post(format!("{}/users/{user_id}/accounts", base()))
        .json(&json!({ "name": unique("acc"), "ledger_symbol": "USD", "code_type": 3 }))
        .send()
        .await
        .unwrap();
    let status = res.status().as_u16();
    let body: Value = res.json().await.unwrap_or(Value::Null);
    assert!(
        status == 201 || status == 202,
        "unexpected status {status}; body {body}"
    );
    assert_eq!(body["account_code_type"], 1);
}

#[tokio::test]
async fn create_account_missing_field_returns_422() {
    let c = client();
    let user_id = create_user(&c).await;
    let res = c
        .post(format!("{}/users/{user_id}/accounts", base()))
        .json(&json!({ "name": unique("acc") }))
        .send()
        .await
        .unwrap();
    expect_status(res, 422).await;
}

#[tokio::test]
async fn fetch_account_returns_it() {
    let c = client();
    let user_id = create_user(&c).await;
    let symbol = create_ledger(&c, 2).await;
    let (_, body) = create_account(&c, &user_id, &symbol).await;
    let account_id = body["account_id"].as_str().unwrap().to_string();

    let fetched = expect_status(
        c.get(format!("{}/accounts/{account_id}", base())).send().await.unwrap(),
        200,
    )
    .await;
    assert_eq!(fetched["account_id"], account_id);
    assert_eq!(fetched["account_code_type"], 1);
    assert_eq!(fetched["account_user_id"], user_id);
}

#[tokio::test]
async fn fetch_unknown_account_returns_404() {
    let c = client();
    let res = c
        .get(format!("{}/accounts/{}", base(), Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    expect_status(res, 404).await;
}

#[tokio::test]
async fn fetch_account_with_malformed_uuid_returns_400() {
    let c = client();
    let res = c
        .get(format!("{}/accounts/nope", base()))
        .send()
        .await
        .unwrap();
    expect_status(res, 400).await;
}

// --- account position (reads balances back out of TigerBeetle) --------------

#[tokio::test]
async fn fetch_position_of_fresh_account_is_all_zero() {
    let c = client();
    let user_id = create_user(&c).await;
    let symbol = create_ledger(&c, 2).await;
    let (_, body) = create_account(&c, &user_id, &symbol).await;
    let account_id = body["account_id"].as_str().unwrap().to_string();

    wait_for_active(&c, &account_id).await;

    let pos = expect_status(
        c.get(format!("{}/accounts/{account_id}/position", base()))
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;

    assert_eq!(pos["account_id"], account_id);
    assert_eq!(pos["symbol"], symbol);
    assert_eq!(pos["decimals"], 2);
    assert_eq!(pos["account_status"], "active");
    // rust_decimal is serialized as a string, scaled to the ledger's decimals
    for field in [
        "debits_posted",
        "credits_posted",
        "debits_pending",
        "credits_pending",
        "net_posted",
    ] {
        assert_eq!(pos[field], "0.00", "{field} should be zero");
    }
}

#[tokio::test]
async fn fetch_position_of_unknown_account_returns_404() {
    let c = client();
    let res = c
        .get(format!("{}/accounts/{}/position", base(), Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    expect_status(res, 404).await;
}

#[tokio::test]
async fn fetch_position_with_malformed_uuid_returns_400() {
    let c = client();
    let res = c
        .get(format!("{}/accounts/xyz/position", base()))
        .send()
        .await
        .unwrap();
    expect_status(res, 400).await;
}

async fn pg() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5433/backend".to_string());
    PgPool::connect(&url).await.expect("connect to postgres")
}

fn tb_client() -> tb::Client {
    let cluster_id = std::env::var("TB_CLUSTER_ID")
        .unwrap_or_else(|_| "0".to_string())
        .parse()
        .expect("parse TB_CLUSTER_ID");
    let addresses =
        std::env::var("TB_IP_ADRESSES").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    tb::Client::new(cluster_id, &addresses).expect("create tigerbeetle client")
}

async fn system_accounts(pool: &PgPool, symbol: &str) -> Vec<(i16, String, Uuid)> {
    sqlx::query_as::<_, (i16, String, Uuid)>(
        "SELECT a.account_code_type, a.account_status::text, a.account_id FROM accounts a \
         JOIN ledgers l ON l.ledger_id = a.account_ledger_id \
         WHERE l.symbol = $1 AND a.account_code_type IN (2, 3) \
         ORDER BY a.account_code_type",
    )
    .bind(symbol)
    .fetch_all(pool)
    .await
    .expect("query system accounts")
}

async fn system_account_id(pool: &PgPool, symbol: &str, code: i16) -> u128 {
    system_accounts(pool, symbol)
        .await
        .into_iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, _, id)| id.as_u128())
        .unwrap_or_else(|| panic!("ledger {symbol} has no system account with code {code}"))
}

async fn assert_system_accounts_ready(pool: &PgPool, tb: &tb::Client, symbol: &str) {
    let accounts = system_accounts(pool, symbol).await;
    let summary: Vec<(i16, &str)> = accounts.iter().map(|(c, s, _)| (*c, s.as_str())).collect();
    assert_eq!(
        summary,
        vec![(2, "active"), (3, "active")],
        "system accounts of {symbol}"
    );

    let ids: Vec<u128> = accounts.iter().map(|(_, _, id)| id.as_u128()).collect();
    let found = tb.lookup_accounts(&ids).await.expect("lookup system accounts");
    assert_eq!(found.len(), 2, "system accounts of {symbol} missing in tigerbeetle");
    for account in found {
        match account.code {
            2 => assert_eq!(
                account.flags,
                tb::AccountFlags::DebitsMustNotExceedCredits,
                "fee account of {symbol}"
            ),
            3 => assert!(account.flags.is_empty(), "source account of {symbol}"),
            other => panic!("unexpected code {other} on a system account of {symbol}"),
        }
    }
}

async fn create_active_account(c: &reqwest::Client, symbol: &str) -> (u128, u32) {
    let user_id = create_user(c).await;
    let (_, body) = create_account(c, &user_id, symbol).await;
    let account_id = body["account_id"].as_str().expect("account_id").to_string();
    let ledger_id = body["account_ledger_id"].as_u64().expect("account_ledger_id") as u32;
    wait_for_active(c, &account_id).await;
    (Uuid::parse_str(&account_id).unwrap().as_u128(), ledger_id)
}

#[tokio::test]
async fn exactly_one_system_user_exists() {
    let pool = pg().await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE is_system")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn seeded_ledgers_have_system_accounts() {
    let pool = pg().await;
    let tb = tb_client();
    for symbol in ["USD", "EUR", "BTC"] {
        assert_system_accounts_ready(&pool, &tb, symbol).await;
    }
}

#[tokio::test]
async fn create_ledger_creates_system_accounts() {
    let c = client();
    let pool = pg().await;
    let tb = tb_client();
    let symbol = create_ledger(&c, 2).await;
    assert_system_accounts_ready(&pool, &tb, &symbol).await;
}

#[tokio::test]
async fn create_ledger_marks_system_outbox_rows_processed() {
    let c = client();
    let pool = pg().await;
    let symbol = create_ledger(&c, 2).await;
    let processed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tb_outbox o \
         JOIN accounts a ON a.account_id = o.aggregate_id \
         JOIN ledgers l ON l.ledger_id = a.account_ledger_id \
         WHERE l.symbol = $1 AND a.account_code_type IN (2, 3) AND o.processed_at IS NOT NULL",
    )
    .bind(&symbol)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(processed, 2);
}

#[tokio::test]
async fn user_account_cannot_overdraw() {
    let c = client();
    let pool = pg().await;
    let tb = tb_client();
    let symbol = create_ledger(&c, 2).await;
    let (user_account, ledger) = create_active_account(&c, &symbol).await;
    let fee_account = system_account_id(&pool, &symbol, 2).await;

    let results = tb
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            debit_account_id: user_account,
            credit_account_id: fee_account,
            amount: 1,
            ledger,
            code: 1,
            ..Default::default()
        }])
        .await
        .expect("send transfer");

    assert_eq!(results.len(), 1, "overdraft transfer should be rejected");
    assert!(
        matches!(results[0].result, tb::CreateTransferResult::ExceedsCredits),
        "unexpected result {:?}",
        results[0].result
    );
}

#[tokio::test]
async fn source_account_funds_user_and_goes_negative() {
    let c = client();
    let pool = pg().await;
    let tb = tb_client();
    let symbol = create_ledger(&c, 2).await;
    let (user_account, ledger) = create_active_account(&c, &symbol).await;
    let source_account = system_account_id(&pool, &symbol, 3).await;

    let results = tb
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            debit_account_id: source_account,
            credit_account_id: user_account,
            amount: 100,
            ledger,
            code: 1,
            ..Default::default()
        }])
        .await
        .expect("send transfer");
    assert!(results.is_empty(), "deposit rejected: {:?}", results[0].result);

    let accounts = tb
        .lookup_accounts(&[source_account, user_account])
        .await
        .expect("lookup accounts");
    let source = accounts.iter().find(|a| a.id == source_account).expect("source account");
    let user = accounts.iter().find(|a| a.id == user_account).expect("user account");
    assert_eq!((source.debits_posted, source.credits_posted), (100, 0));
    assert_eq!((user.debits_posted, user.credits_posted), (0, 100));

    let pos = expect_status(
        c.get(format!(
            "{}/accounts/{}/position",
            base(),
            Uuid::from_u128(user_account)
        ))
        .send()
        .await
        .unwrap(),
        200,
    )
    .await;
    assert_eq!(pos["net_posted"], "1.00");
}
