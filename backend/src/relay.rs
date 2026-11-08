use crate::{
    domain::AccountCodeType,
    v1::{AppState, MarketVerdict, activate_market, advance_market, advance_order, remove_market},
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
                        flags: m.code.tb_flags(),
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

// The order relay does for orders what relay_loop does for accounts. it finishes what the backend
// couldnt do. Each pending order_outbox row is an order that still
// needs something: its hold account and reservation, the PlaceOrder, or the refund of a rejection.
// advance_order works out which from the order itself and is safe to repeat, so this just calls it
// until the order is finished and then stamps the row. The pending orders are taken in
// turns (the least recently tried first), so the ones that never finish don't hold up the rest
pub async fn order_relay_loop(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        order_relay_tick(&state).await;
    }
}

// one pass of order_relay_loop
pub async fn order_relay_tick(state: &AppState) {
    let tx = state.pg_connections.begin().await;
    if let Err(e) = tx {
        println!("RELAY: order relay couldn't establish a connection to the db: {e}");
        return;
    };
    let mut tx = tx.unwrap();
    // 15 seconds of grace,
    // Only three orders at a time: each one can wait on the network
    // and this transaction and its row locks are held meanwhile. skip locked lets several backends share the work
    let pending = sqlx::query_scalar::<_, Uuid>(
        "SELECT order_id FROM order_outbox \
         WHERE processed_at IS NULL \
         AND created_at < now() - interval '15 seconds' \
         ORDER BY last_attempt_at NULLS FIRST, id LIMIT 3 \
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_all(&mut *tx)
    .await;
    let pending = match pending {
        Ok(v) if v.is_empty() => return,
        Ok(v) => v,
        Err(e) => {
            println!("RELAY: couldn't drain orders from db: {e}");
            return;
        }
    };

    let mut finished: Vec<Uuid> = Vec::new();
    for &order_id in &pending {
        match advance_order(state, order_id, 1).await {
            Ok(advance) if advance.finished => finished.push(order_id),
            Ok(_) => {}
            Err(code) => println!("RELAY: order {order_id} couldn't be advanced: {code}"),
        }
    }

    let marking = sqlx::query(
        "UPDATE order_outbox SET last_attempt_at = now(), \
         processed_at = CASE WHEN order_id = ANY($2) THEN now() ELSE processed_at END \
         WHERE order_id = ANY($1)",
    )
    .bind(&pending)
    .bind(finished)
    .execute(&mut *tx)
    .await;
    if let Err(e) = marking {
        println!("RELAY: couldn't stamp the outbox's finished orders: {e}");
        return;
    }
    if let Err(e) = tx.commit().await {
        println!("RELAY: couldn't commit to db: {e}");
    }
}

// relay for markets. We need one because of all the network calls
pub async fn market_relay_loop(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        market_relay_tick(&state).await;
    }
}

// one pass of market_relay_loop
pub async fn market_relay_tick(state: &AppState) {
    let tx = state.pg_connections.begin().await;
    if let Err(e) = tx {
        println!("RELAY: market relay couldn't establish a connection to the db: {e}");
        return;
    };
    let mut tx = tx.unwrap();
    let pending = sqlx::query_scalar::<_, i32>(
        "SELECT market_id FROM market_outbox \
         WHERE processed_at IS NULL \
         AND created_at < now() - interval '15 seconds' \
         ORDER BY last_attempt_at NULLS FIRST, id LIMIT 3 \
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_all(&mut *tx)
    .await;
    let pending = match pending {
        Ok(v) if v.is_empty() => return,
        Ok(v) => v,
        Err(e) => {
            println!("RELAY: couldn't drain markets from db: {e}");
            return;
        }
    };

    for &market_id in &pending {
        match advance_market(state, market_id).await {
            Ok(MarketVerdict::Accepted) => {
                if let Err(e) = activate_market(&mut *tx, market_id).await {
                    println!("RELAY: couldn't activate market {market_id}: {e}");
                }
            }
            Ok(MarketVerdict::Refused) => {
                if let Err(e) = remove_market(&mut tx, market_id).await {
                    println!("RELAY: couldn't remove the refused market {market_id}: {e}");
                }
            }
            Ok(_) => {}
            Err(code) => println!("RELAY: market {market_id} couldn't be advanced: {code}"),
        }
    }

    let marking =
        sqlx::query("UPDATE market_outbox SET last_attempt_at = now() WHERE market_id = ANY($1)")
            .bind(&pending)
            .execute(&mut *tx)
            .await;
    if let Err(e) = marking {
        println!("RELAY: couldn't stamp the tried markets: {e}");
        return;
    }
    if let Err(e) = tx.commit().await {
        println!("RELAY: couldn't commit to db: {e}");
    }
}
