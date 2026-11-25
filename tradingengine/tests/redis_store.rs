//! `RedisStore` on its own. How the engine uses it, saving before every answer, is tested
//! in `tests/router_consumer.rs`.
//!
//! These need a running Redis at `REDIS_URL`, so they're `#[ignore]`d. CI starts a Redis and
//! runs them with `-- --ignored`.

use tradingengine::redis_store::RedisStore;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_instance_that_never_saved_loads_nothing() {
    let mut store = RedisStore::connect(&redis_url(), "test-never-saved")
        .await
        .expect("connect");
    assert_eq!(store.load_state().await.expect("load"), None);
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn saved_bytes_come_back_unchanged() {
    let mut store = RedisStore::connect(&redis_url(), "test-round-trip")
        .await
        .expect("connect");
    let bytes = vec![0, 1, 2, 255, 0];
    store.save_state(&bytes).await.expect("save");
    assert_eq!(store.load_state().await.expect("load"), Some(bytes));
}
