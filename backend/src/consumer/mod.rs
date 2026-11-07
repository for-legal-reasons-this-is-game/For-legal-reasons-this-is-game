use crate::service::EngineClient;
use cn_tigerbeetle as tb;
use plan::{BatchPlan, LimitMemory, Lookups};
use proto::{EventItem, SubscribeEventsRequest};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use stream::{SourceSettings, Stats, run_source};
use tokio_util::sync::CancellationToken;

pub mod ids;
pub mod ledger;
pub mod lock;
pub mod pipeline;
pub mod plan;
pub mod store;
pub mod stream;

pub type ConsumerError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct Config {
    // where the events come from. The router sits behind this url, like it does for the backend
    pub engine_url: String,
    pub database_url: String,
    pub tb_cluster_id: u128,
    pub tb_addresses: String,
    pub batch_max_items: usize,
    pub batch_max_wait: Duration,
    pub queue_depth: usize,
    // key of the postgres advisory lock that keeps a second consumer from starting
    pub lock_key: i64,
}

// "tradecon" as bytes
pub const DEFAULT_LOCK_KEY: i64 = 0x7472_6164_6563_6F6E;

impl Config {
    pub fn from_env() -> Result<Config, ConsumerError> {
        Config::from_lookup(|k| std::env::var(k).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Config, ConsumerError> {
        let required = |k: &str| get(k).ok_or_else(|| format!("{k} must be set"));
        let number = |k: &str, default: u64| -> Result<u64, ConsumerError> {
            match get(k) {
                None => Ok(default),
                Some(v) => v
                    .parse()
                    .map_err(|_| format!("{k} must be a number, got {v:?}").into()),
            }
        };
        let tb_cluster_id = required("TB_CLUSTER_ID")?;
        Ok(Config {
            engine_url: required("ENGINE_URL")?,
            database_url: required("DATABASE_URL")?,
            tb_cluster_id: tb_cluster_id
                .parse()
                .map_err(|_| format!("TB_CLUSTER_ID must be a number, got {tb_cluster_id:?}"))?,
            tb_addresses: required("TB_IP_ADRESSES")?,
            batch_max_items: number("BATCH_MAX_ITEMS", 500)? as usize,
            batch_max_wait: Duration::from_millis(number("BATCH_MAX_WAIT_MS", 50)?),
            queue_depth: number("QUEUE_DEPTH", 4)? as usize,
            lock_key: DEFAULT_LOCK_KEY,
        })
    }
}

// reads what postgres knows about the batch (three lookups at once) and plans it
async fn prepare_batch(
    pool: &PgPool,
    limits: &Mutex<LimitMemory>,
    items: Vec<EventItem>,
) -> Result<BatchPlan, ConsumerError> {
    let (trade_ids, order_ids, symbols) = plan::wanted_ids(&items);
    let (known_trades, orders, markets) = tokio::try_join!(
        store::known_trade_ids(pool, &trade_ids),
        store::load_orders(pool, &order_ids),
        store::load_markets(pool, &symbols),
    )?;
    let lookups = Lookups {
        orders,
        markets,
        known_trades,
    };
    // there is no await below, so the lock is never held while waiting
    let mut memory = limits.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(plan::build_plan(&items, &lookups, &mut memory)?)
}

// Runs the consumer until `cancel` fires (graceful stop, Ok) or a stage fails (Err: the process
// should exit non-zero and be restarted). Takes the single-consumer lock first.
pub async fn run(cfg: Config, cancel: CancellationToken) -> Result<(), ConsumerError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;
    let lock = lock::try_lock(&cfg.database_url, cfg.lock_key).await?;
    let result = run_locked(&cfg, &pool, cancel).await;
    lock.release().await;
    result
}

async fn run_locked(
    cfg: &Config,
    pool: &PgPool,
    cancel: CancellationToken,
) -> Result<(), ConsumerError> {
    let tb_client = Arc::new(
        tb::Client::new(cfg.tb_cluster_id, &cfg.tb_addresses)
            .map_err(|e| format!("tigerbeetle client could not start: {e:?}"))?,
    );
    let engine = EngineClient::with_keepalive(
        &cfg.engine_url,
        Duration::from_secs(20),
        Duration::from_secs(10),
    )?;
    let stats = Arc::new(Stats::default());
    let settings = SourceSettings::from_config(cfg);

    // the limit prices seen so far, shared by all the batches (see LimitMemory)
    let limits = Arc::new(Mutex::new(LimitMemory::default()));
    let prepare = {
        let pool = pool.clone();
        move |items: Vec<EventItem>| {
            let (pool, limits) = (pool.clone(), limits.clone());
            async move { prepare_batch(&pool, &limits, items).await }
        }
    };
    // money first: a crash after this and before the commit is repaired by the replay, because
    // the transfer ids are deterministic and tigerbeetle answers `exists`
    let settle = {
        let tb_client = tb_client.clone();
        move |plan: BatchPlan| {
            let tb_client = tb_client.clone();
            async move {
                ledger::send(&tb_client, &plan.transfers).await?;
                Ok::<_, ConsumerError>(plan)
            }
        }
    };
    let commit = {
        let pool = pool.clone();
        move |plan: BatchPlan| {
            let pool = pool.clone();
            async move {
                let done = store::commit(&pool, &plan.trades, &plan.order_updates).await?;
                println!("CONSUMER: batch committed: {done:?}");
                Ok::<_, ConsumerError>(())
            }
        }
    };

    let source_cancel = cancel.clone();
    pipeline::run_pipeline(
        move |events| {
            run_source(
                move || {
                    let engine = engine.clone();
                    async move { engine.subscribe_events(SubscribeEventsRequest {}).await }
                },
                settings,
                events,
                source_cancel,
                stats,
            )
        },
        prepare,
        settle,
        commit,
        cfg.queue_depth,
        cancel,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    const REQUIRED: [(&str, &str); 4] = [
        ("ENGINE_URL", "http://tradingengine:50051"),
        ("DATABASE_URL", "postgres://x"),
        ("TB_CLUSTER_ID", "0"),
        ("TB_IP_ADRESSES", "127.0.0.1:3000"),
    ];

    #[test]
    fn config_defaults_when_optional_vars_missing() {
        let cfg = Config::from_lookup(lookup(&REQUIRED)).unwrap();
        assert_eq!(cfg.engine_url, "http://tradingengine:50051");
        assert_eq!(cfg.batch_max_items, 500);
        assert_eq!(cfg.batch_max_wait, Duration::from_millis(50));
        assert_eq!(cfg.queue_depth, 4);
        assert_eq!(cfg.lock_key, DEFAULT_LOCK_KEY);
    }

    #[test]
    fn config_reads_overrides() {
        let mut pairs = REQUIRED.to_vec();
        pairs.extend([
            ("BATCH_MAX_ITEMS", "10"),
            ("BATCH_MAX_WAIT_MS", "5"),
            ("QUEUE_DEPTH", "2"),
        ]);
        let cfg = Config::from_lookup(lookup(&pairs)).unwrap();
        assert_eq!(cfg.batch_max_items, 10);
        assert_eq!(cfg.batch_max_wait, Duration::from_millis(5));
        assert_eq!(cfg.queue_depth, 2);
    }

    #[test]
    fn config_missing_engine_url_is_error() {
        let pairs: Vec<_> = REQUIRED
            .iter()
            .copied()
            .filter(|(k, _)| *k != "ENGINE_URL")
            .collect();
        let err = Config::from_lookup(lookup(&pairs)).unwrap_err();
        assert!(err.to_string().contains("ENGINE_URL"), "{err}");
    }

    #[test]
    fn config_bad_number_is_error() {
        let mut pairs = REQUIRED.to_vec();
        pairs.push(("QUEUE_DEPTH", "abc"));
        assert!(Config::from_lookup(lookup(&pairs)).is_err());
    }
}
