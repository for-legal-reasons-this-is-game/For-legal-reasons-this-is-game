use crate::{
    domain::AccountCodeType,
    v1::AppState,
};
use cn_tigerbeetle::{self as tb};
use sqlx::{self};
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct TbOutbox {
    id: i64,
    aggregate_id: Uuid,
    ledger: i32,
    code: AccountCodeType,
    user_id: Uuid,
}
// The relay gets launched as a working thread, drains postgres' outbox, creates the account for tb.
// basically, it polls every, more or less, second.
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
                let tb_accounts = timeout(
                    Duration::from_secs(5),
                    state.tb_client.create_accounts(&accounts),
                )
                .await;

                let tb_accounts = match tb_accounts {
                    Err(_elapsed) => {
                        println!(
                            "RELAY: Tigerbeetle didn't respond within 5 seconds, releasing the batch and trying again"
                        );
                        continue;
                    }
                    Ok(Err(e)) => {
                        println!("RELAY: Connection to cn_tigerbeetle failed: {e}");
                        continue;
                    }
                    Ok(Ok(v)) => v,
                };

                let mut bad_indices: Vec<usize> = Vec::new();

                for writes in tb_accounts {
                    match writes.result {
                        tb::CreateAccountResult::Ok | tb::CreateAccountResult::Exists => {}
                        e => {
                            println!(
                                "RELAY: failed to create account with batch index {}: {e}",
                                writes.index
                            );
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

                //mark both status and outbox
                let marking = sqlx::query(
                    "WITH marked AS ( \
                    UPDATE tb_outbox SET processed_at = now() \
                    WHERE id = ANY($1) \
                    RETURNING aggregate_id
                    ) \
                    UPDATE accounts SET account_status = 'active' \
                    WHERE account_id IN (SELECT aggregate_id FROM marked)",
                )
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
