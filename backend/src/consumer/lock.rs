// Only one consumer may run: two would settle the same events at once. Kubernetes runs one
// replica with the Recreate strategy; this lock is the second guard if a rollout ever overlaps.
// It is a Postgres advisory lock held by a dedicated connection, so it vanishes if the holder dies.
use crate::consumer::ConsumerError;
use sqlx::{Connection, PgConnection};

pub struct LockGuard {
    conn: PgConnection,
}

pub async fn try_lock(database_url: &str, key: i64) -> Result<LockGuard, ConsumerError> {
    let mut conn = PgConnection::connect(database_url).await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(key)
        .fetch_one(&mut conn)
        .await?;
    if !acquired {
        return Err("another consumer holds the advisory lock".into());
    }
    Ok(LockGuard { conn })
}

impl LockGuard {
    // closes the connection politely; dropping the guard also releases the lock, a little later
    pub async fn release(self) {
        let _ = self.conn.close().await;
    }
}
