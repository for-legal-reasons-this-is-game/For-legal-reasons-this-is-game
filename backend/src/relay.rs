use uuid::Uuid;

use crate::{
    domain::{AccountCodeType, LedgerType},
    v1::AppState,
};
use sqlx::{self};
use tokio::time::Duration;

#[derive(sqlx::FromRow)]
struct TbOutbox {
    id: i64,
    account_id: Uuid,
    ledger: LedgerType,
    code: AccountCodeType,
    user_id: Uuid,
}

async fn relay_loop(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let mut tx = state.pg_connections.begin().await;
        if let Err(e) = tx {
            println!("RELAY: relay couldn't establish a connection to the db: {e}");
            continue;
        };
        let mut tx = tx.unwrap();
        match sqlx::query_as::<_, TbOutbox>(
            "SELECT id, aggregate_id, ledger, code, user_id FROM tb_outbox\
        WHERE processed_at IS NULL ORDER BY id LIMIT 100 \
        FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await
        {
            Ok(v) => {
                todo!()
            }
            Err(e) => {
                println!("RELAY: couldn't drain from db: {e}");
                continue;
            }
        }
    }
}
