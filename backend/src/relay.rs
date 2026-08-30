use crate::{
    domain::{AccountCodeType, LedgerType},
    v1::AppState,
};
use cn_tigerbeetle::{self as tb};
use sqlx::{self};
use tokio::time::Duration;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct TbOutbox {
    id: i64,
    aggregate_id: Uuid,
    ledger: LedgerType,
    code: AccountCodeType,
    user_id: Uuid,
}
// The relay gets launched as a working thread, drains postgres' outbox, creates the account for tb.
// basically, it polls every, more or less, second.
// eventually we'll also like to prune the outbox every couple of hours.
pub async fn relay_loop(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let tx = state.pg_connections.begin().await;
        if let Err(e) = tx {
            println!("RELAY: relay couldn't establish a connection to the db: {e}");
            continue;
        };
        let mut tx = tx.unwrap();
        //5 seconds for the grace period so that the endpoint can try to self-report
        match sqlx::query_as::<_, TbOutbox>(
            "SELECT id, aggregate_id, ledger, code, user_id FROM tb_outbox \
        WHERE processed_at IS NULL \
        AND  created_at < now() - interval '5 seconds' \
        ORDER BY id LIMIT 100 \
        FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await
        {
            Ok(v) => {
                if v.is_empty() {
                    continue;
                }
                let accounts: Vec<tb::Account> = v
                    .iter()
                    .map(|m| tb::Account {
                        id: m.aggregate_id.as_u128(),
                        ledger: m.ledger as u32,
                        code: m.code as u16,
                        user_data_128: m.user_id.as_u128(),
                        ..Default::default()
                    })
                    .collect();
                let tb_accounts = state.tb_client.create_accounts(&accounts).await;
                if let Err(e) = tb_accounts {
                    println!("RELAY: connection to cn_tigerbeetle failed: {e}");
                    continue;
                }
                let mut bad_indices: Vec<usize> = Vec::new();
                for writes in tb_accounts.unwrap() {
                    match writes.result {
                        tb::CreateAccountResult::Ok | tb::CreateAccountResult::Exists => {}
                        _ => {
                            bad_indices.push(writes.index);
                        }
                    }
                }
                let processed_ids: Vec<i64> = v
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !bad_indices.contains(i))
                    .map(|(_, row)| row.id)
                    .collect();
                let marking =
                    sqlx::query("UPDATE tb_outbox SET processed_at = now() WHERE id = ANY($1)")
                        .bind(processed_ids)
                        .execute(&mut *tx)
                        .await;
                if let Err(e) = marking {
                    println!("RELAY: couldn't stamp the outbox's completed rows: {e}");
                    continue;
                }
                if let Err(e) = tx.commit().await {
                    println!("Couldn't commit to db: {e}");
                }
            }
            Err(e) => {
                println!("RELAY: couldn't drain from db: {e}");
                continue;
            }
        }
    }
}
