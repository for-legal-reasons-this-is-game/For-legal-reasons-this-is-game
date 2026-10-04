use crate::{
    domain::{AccountCodeType, Ledger},
    v1::AppState,
};
use cn_tigerbeetle as tb;
use sqlx::PgConnection;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

pub type BootstrapError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run(state: &AppState) -> Result<(), BootstrapError> {
    let ledgers: Vec<Ledger> = {
        let cache = state
            .ledgers
            .read()
            .map_err(|_| "ledger cache lock poisoned")?;
        cache.values().cloned().collect()
    };

    for ledger in &ledgers {
        let mut tx = state.pg_connections.begin().await?;
        insert_system_accounts(&mut tx, ledger.ledger_id).await?;
        tx.commit().await?;

        if !activate_system_accounts(state, ledger.ledger_id).await? {
            return Err(format!(
                "system accounts for ledger {} couldn't be created in tigerbeetle",
                ledger.symbol
            )
            .into());
        }
    }
    Ok(())
}

pub async fn insert_system_accounts(
    conn: &mut PgConnection,
    ledger_id: i32,
) -> Result<(), sqlx::Error> {
    let system_user = ensure_system_user(conn).await?;

    for code in [AccountCodeType::Fee, AccountCodeType::Source] {
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO accounts (account_id, account_name, account_ledger_id, account_code_type, account_user_id) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (account_ledger_id, account_code_type) WHERE account_code_type IN (2, 3) DO NOTHING \
             RETURNING account_id",
        )
        .bind(Uuid::from_u128(tb::id()))
        .bind(format!("system {code:?}").to_lowercase())
        .bind(ledger_id)
        .bind(code)
        .bind(system_user)
        .fetch_optional(&mut *conn)
        .await?;

        if let Some(account_id) = inserted {
            sqlx::query(
                "INSERT INTO tb_outbox(aggregate_id, ledger, code, user_id) VALUES ($1, $2, $3, $4)",
            )
            .bind(account_id)
            .bind(ledger_id)
            .bind(code)
            .bind(system_user)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn activate_system_accounts(state: &AppState, ledger_id: i32) -> Result<bool, sqlx::Error> {
    let pending = sqlx::query_as::<_, (Uuid, AccountCodeType, Uuid)>(
        "SELECT account_id, account_code_type, account_user_id FROM accounts \
         WHERE account_ledger_id = $1 AND account_code_type IN (2, 3) AND account_status = 'processing'",
    )
    .bind(ledger_id)
    .fetch_all(&state.pg_connections)
    .await?;

    let mut all_active = true;
    for (account_id, code, user_id) in pending {
        let account = tb::Account {
            id: account_id.as_u128(),
            ledger: ledger_id as u32,
            code: code.into(),
            user_data_128: user_id.as_u128(),
            flags: code.tb_flags(),
            ..Default::default()
        };

        let created = match timeout(
            Duration::from_millis(500),
            state.tb_client.create_accounts(&[account]),
        )
        .await
        {
            Ok(Ok(results)) => match results.first() {
                None => true,
                Some(r) if matches!(r.result, tb::CreateAccountResult::Exists) => true,
                Some(r) => {
                    println!(
                        "BOOTSTRAP: tigerbeetle rejected system account {account_id}. Relay will pick it up: {:?}",
                        r.result
                    );
                    false
                }
            },
            Ok(Err(e)) => {
                println!("BOOTSTRAP: connection to tigerbeetle failed. Relay will attempt it: {e}");
                false
            }
            Err(_) => {
                println!("BOOTSTRAP: tigerbeetle timed out. Relay will pick it up.");
                false
            }
        };

        if !created {
            all_active = false;
            continue;
        }

        sqlx::query(
            "WITH marked AS ( \
                 UPDATE tb_outbox SET processed_at = now() \
                 WHERE aggregate_id = $1 AND processed_at IS NULL \
                 RETURNING aggregate_id \
             ) \
             UPDATE accounts SET account_status = 'active' WHERE account_id = $1",
        )
        .bind(account_id)
        .execute(&state.pg_connections)
        .await?;
    }
    Ok(all_active)
}

async fn ensure_system_user(conn: &mut PgConnection) -> Result<Uuid, sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (user_name, is_system) VALUES ('system', TRUE) \
         ON CONFLICT (is_system) WHERE is_system DO NOTHING",
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM users WHERE is_system")
        .fetch_one(&mut *conn)
        .await
}
