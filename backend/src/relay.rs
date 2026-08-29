use uuid::Uuid;

use crate::{
    domain::{AccountCodeType, LedgerType},
    v1::AppState,
};
use tokio::time::{Duration, Interval};

#[derive(sqlx::FromRow)]
struct TbOutbox {
    id: i64,
    account_id: Uuid,
    ledger: LedgerType,
    code: AccountCodeType,
    user_id: Uuid,
}
async fn relay_loop(state: AppState) {
    todo!();
    /*
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            let tx = state.pg_connections.begin().await;
            interval.tick().await;
            let rows: Vec<TbOutbox> = query_as::<_, TbOutbox>(
                "SELECT id, aggregate_id, ledger, code, user_id FROM tb_outbox\
            WHERE processed_at IS NULL ORDER BY id LIMIT 100 \
            FOR UPDATE SKIP LOCKED",
            )
            .fetch_all(&mut *tx)
            .await;
        }
    */
}
