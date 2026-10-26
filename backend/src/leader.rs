// With several backend pods, only one of them may read the engine's event stream: it's one ordered
// stream with one cursor. Postgres decides who: an advisory lock is held by a database connection and
// is released by postgres the moment that connection dies, even if the pod crashed. The pods that
// don't get it keep trying every few seconds (hot standbys).
use tokio::sync::watch;
use tokio::time::{Duration, sleep, timeout};

use sqlx::{Connection, PgConnection, PgPool};

use crate::settlement::StreamError;
use crate::v1::AppState;
use crate::{consumer, recovery};

// any number works as long as every pod uses the same one
const CONSUMER_LOCK_KEY: i64 = 7_262_001;
const RETRY_WAIT: Duration = Duration::from_secs(5);
const HEARTBEAT_WAIT: Duration = Duration::from_secs(5);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

pub async fn run(state: AppState, mut stop: watch::Receiver<bool>) {
    while !*stop.borrow() {
        if let Err(e) = lead_once(&state, &mut stop).await {
            eprintln!("LEADER: {e}");
        }
        tokio::select! {
            _ = sleep(RETRY_WAIT) => {}
            _ = stop.changed() => {}
        }
    }
}

// Tries to take the lock. The lock belongs to the connection that took it, so that connection can't
// come from the pool: a pooled connection would be handed to other queries and returned with the lock
// still on it. detach() takes it out of the pool for good, and closing it releases the lock.
async fn acquire_lock(pool: &PgPool) -> Result<Option<PgConnection>, sqlx::Error> {
    let mut conn: PgConnection = pool.acquire().await?.detach();
    let got_it: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(CONSUMER_LOCK_KEY)
        .fetch_one(&mut conn)
        .await?;
    if got_it {
        Ok(Some(conn))
    } else {
        let _ = conn.close().await;
        Ok(None)
    }
}

// tries to become the leader. Returns when this pod isn't the leader, or stopped being it.
async fn lead_once(state: &AppState, stop: &mut watch::Receiver<bool>) -> Result<(), StreamError> {
    let Some(mut lock_conn) = acquire_lock(&state.pg_connections).await? else {
        return Ok(());
    };
    println!("LEADER: this pod reads the engine's event stream now");

    // each task is spawned on its own so a panic in one can't take this loop down with it
    let mut consumer_task = tokio::spawn(consumer::run(state.clone(), stop.clone()));
    let mut recovery_task = tokio::spawn(recovery::run(state.clone(), stop.clone()));

    tokio::select! {
        result = &mut consumer_task => eprintln!("LEADER: the consumer ended: {result:?}"),
        result = &mut recovery_task => eprintln!("LEADER: the recovery job ended: {result:?}"),
        result = heartbeat(&mut lock_conn) => {
            eprintln!("LEADER: lost the lock connection, stepping down: {result:?}")
        }
        _ = stop.changed() => {
            // shutting down: the tasks watch the same signal, give them time to finish their batch
            let finished = timeout(SHUTDOWN_GRACE, async {
                let _ = (&mut consumer_task).await;
                let _ = (&mut recovery_task).await;
            })
            .await;
            if finished.is_err() {
                eprintln!("LEADER: the tasks didn't stop in time, aborting them");
            }
        }
    }

    // stepping down: nothing may keep working once the lock is gone. Aborting a finished task is fine.
    consumer_task.abort();
    recovery_task.abort();
    let _ = lock_conn.close().await;
    Ok(())
}

// Checks the lock connection is alive. It only ever returns with an error: a dead connection means
// postgres already released the lock and somebody else may be the leader already.
async fn heartbeat(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    loop {
        sleep(HEARTBEAT_WAIT).await;
        sqlx::query("SELECT 1").execute(&mut *conn).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::test_db::TestDb;

    #[tokio::test]
    async fn only_one_pod_holds_the_lock_and_a_dead_one_gives_it_up() {
        let Some(db) = TestDb::new().await else {
            return;
        };

        let first = acquire_lock(&db.pool).await.unwrap();
        assert!(first.is_some());
        // a standby doesn't get it, and doesn't wait for it either
        assert!(acquire_lock(&db.pool).await.unwrap().is_none());
        assert!(acquire_lock(&db.pool).await.unwrap().is_none());

        // the leader's connection dies (a crashed pod): postgres frees the lock by itself, and the
        // heartbeat of the leader notices
        let mut first = first.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut first)
            .await
            .unwrap();
        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .execute(&db.pool)
            .await
            .unwrap();
        assert!(heartbeat(&mut first).await.is_err());

        let second = acquire_lock(&db.pool).await.unwrap();
        assert!(second.is_some());

        // closing the connection (a pod that steps down) is enough as well
        second.unwrap().close().await.unwrap();
        let third = acquire_lock(&db.pool).await.unwrap();
        assert!(third.is_some());
        third.unwrap().close().await.unwrap();
        db.cleanup().await;
    }
}
