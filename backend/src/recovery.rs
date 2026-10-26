// Jobs that only the leader runs, for orders the normal flow left half done. A pod can die between
// the steps of placing an order, and nothing else would ever finish them.
use proto::PlaceOrderRequest;
use sqlx::PgPool;
use tokio::sync::watch;
use tokio::time::{Duration, MissedTickBehavior, interval, timeout};
use uuid::Uuid;

use crate::settlement::{StreamError, refund_orders};
use crate::v1::AppState;

const TICK: Duration = Duration::from_secs(15);
const DUPLICATE_COMMAND: i32 = 8; // proto RejectReason

pub async fn run(state: AppState, mut stop: watch::Receiver<bool>) -> Result<(), StreamError> {
    let mut ticker = interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = stop.changed() => return Ok(()),
        }
        // a failure here is retried on the next tick, so it never stops the leader
        if let Err(e) = release_stale_reservations(&state).await {
            eprintln!("RECOVERY: stale reservations: {e}");
        }
        if let Err(e) = resend_unconfirmed(&state).await {
            eprintln!("RECOVERY: unconfirmed orders: {e}");
        }
    }
}

// An order stuck in `reserving` was never sent to the engine (the pod died after saving it, or while
// moving the funds). Whatever it reserved goes back to the user and the order is rejected.
async fn release_stale_reservations(state: &AppState) -> Result<(), StreamError> {
    let stale = stale_reservations(&state.pg_connections).await?;
    if stale.is_empty() {
        return Ok(());
    }
    println!(
        "RECOVERY: releasing {} orders that never left `reserving`",
        stale.len()
    );

    // the hold account may not exist yet if the pod died before creating it
    refund_orders(state, &stale, true).await?;
    sqlx::query(
        "UPDATE orders SET submission = 'rejected', \
             reject_detail = 'the reservation was never completed' \
         WHERE order_id = ANY($1) AND submission = 'reserving'",
    )
    .bind(&stale)
    .execute(&state.pg_connections)
    .await?;
    Ok(())
}

async fn stale_reservations(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT order_id FROM orders \
         WHERE submission = 'reserving' AND created_at < now() - interval '2 minutes' \
         ORDER BY created_at LIMIT 100",
    )
    .fetch_all(pool)
    .await
}

#[derive(sqlx::FromRow)]
struct UnconfirmedOrder {
    order_id: Uuid,
    command_id: Uuid,
    user_id: Uuid,
    symbol: String,
    base_account_id: Uuid,
    quote_account_id: Uuid,
    side: i16,
    order_type: i16,
    time_in_force: i16,
    base_quantity: Option<String>,
    quote_quantity: Option<String>,
    limit_price: Option<String>,
    trigger_price: Option<String>,
    trigger_direction: Option<i16>,
    protection_price: Option<String>,
    expire_seconds: Option<i64>,
    expire_micros: Option<i64>,
    maker_fee_bps: i32,
    taker_fee_bps: i32,
}

// sent a while ago and neither the response nor the stream confirmed them
async fn unconfirmed_orders(pool: &PgPool) -> Result<Vec<UnconfirmedOrder>, sqlx::Error> {
    sqlx::query_as::<_, UnconfirmedOrder>(
        "SELECT o.order_id, o.command_id, o.user_id, m.symbol, o.base_account_id, o.quote_account_id, \
                o.side, o.order_type, o.time_in_force, \
                o.base_quantity::text AS base_quantity, o.quote_quantity::text AS quote_quantity, \
                o.limit_price::text AS limit_price, o.trigger_price::text AS trigger_price, \
                o.trigger_direction, o.protection_price::text AS protection_price, \
                floor(extract(epoch FROM o.expire_time))::bigint AS expire_seconds, \
                (extract(microseconds FROM o.expire_time)::bigint % 1000000) AS expire_micros, \
                o.maker_fee_bps, o.taker_fee_bps \
         FROM orders o JOIN markets m ON m.market_id = o.market_id \
         WHERE o.submission = 'sent' AND o.status IS NULL \
           AND o.updated_at < now() - interval '30 seconds' \
         ORDER BY o.created_at LIMIT 50",
    )
    .fetch_all(pool)
    .await
}

fn uint(value: &str, name: &str) -> Result<proto::Uint128, String> {
    value
        .parse::<u128>()
        .map(proto::Uint128::from)
        .map_err(|_| format!("unreadable {name}"))
}

fn optional_uint(value: &Option<String>, name: &str) -> Result<Option<proto::Uint128>, String> {
    value.as_deref().map(|v| uint(v, name)).transpose()
}

// The same command again, rebuilt from the order. The engine knows the command_id, so if it did
// get the first one it answers with what it answered then.
fn place_order_request(o: &UnconfirmedOrder) -> Result<PlaceOrderRequest, String> {
    use proto::place_order_request::Amount;
    let amount = match (&o.base_quantity, &o.quote_quantity) {
        (Some(base), None) => Amount::BaseQuantity(uint(base, "base_quantity")?),
        (None, Some(quote)) => Amount::QuoteQuantity(uint(quote, "quote_quantity")?),
        _ => return Err("the order doesn't have exactly one quantity".into()),
    };
    let expire_time = match (o.expire_seconds, o.expire_micros) {
        (Some(seconds), Some(micros)) => Some(proto::Timestamp {
            seconds,
            nanos: (micros * 1000) as i32,
        }),
        _ => None,
    };
    Ok(PlaceOrderRequest {
        command_id: Some(o.command_id.as_u128().into()),
        order_id: Some(o.order_id.as_u128().into()),
        market_symbol: o.symbol.clone(),
        account_holder: Some(o.user_id.as_u128().into()),
        base_account: Some(o.base_account_id.as_u128().into()),
        quote_account: Some(o.quote_account_id.as_u128().into()),
        side: i32::from(o.side),
        r#type: i32::from(o.order_type),
        time_in_force: i32::from(o.time_in_force),
        amount: Some(amount),
        limit_price: optional_uint(&o.limit_price, "limit_price")?,
        trigger_price: optional_uint(&o.trigger_price, "trigger_price")?,
        trigger_direction: o.trigger_direction.map_or(0, i32::from),
        protection_price: optional_uint(&o.protection_price, "protection_price")?,
        expire_time,
        sent_at: Some(std::time::SystemTime::now().into()),
        maker_fee_bps: o.maker_fee_bps as u32,
        taker_fee_bps: o.taker_fee_bps as u32,
    })
}

// An order that was sent but never confirmed, neither by the response nor by the stream. Either the
// engine has it (and the answer is the same as the first time) or the command never got there, and
// this places it. Retrying with the same command_id is what the guide asks for after a timeout.
// Nothing is retried while the engine is being recovered from a restart, that procedure refunds them.
async fn resend_unconfirmed(state: &AppState) -> Result<(), StreamError> {
    let accepting = sqlx::query_scalar::<_, bool>(
        "SELECT accepting_orders FROM engine_stream_state WHERE id = 1",
    )
    .fetch_one(&state.pg_connections)
    .await?;
    if !accepting {
        return Ok(());
    }

    let orders = unconfirmed_orders(&state.pg_connections).await?;

    for order in orders {
        let request = match place_order_request(&order) {
            Ok(request) => request,
            Err(why) => {
                eprintln!(
                    "RECOVERY: can't rebuild the command of order {}: {why}",
                    order.order_id
                );
                continue;
            }
        };
        let response = match timeout(
            Duration::from_secs(5),
            state.engine_client.place_order(request),
        )
        .await
        {
            Ok(Ok(response)) => response,
            // not sure what the engine got: leave it for the next tick
            Ok(Err(status)) => {
                eprintln!(
                    "RECOVERY: resending order {} failed: {status}",
                    order.order_id
                );
                continue;
            }
            Err(_) => {
                eprintln!("RECOVERY: resending order {} timed out", order.order_id);
                continue;
            }
        };

        match response.reject_reason {
            0 => {
                // only `submission`: the status and the fills are written by the event stream
                sqlx::query("UPDATE orders SET submission = 'accepted' WHERE order_id = $1 AND submission = 'sent'")
                    .bind(order.order_id)
                    .execute(&state.pg_connections)
                    .await?;
            }
            DUPLICATE_COMMAND => eprintln!(
                "RECOVERY: the engine says order {} is a different command with the same id, needs a human",
                order.order_id
            ),
            reason => {
                refund_orders(state, &[order.order_id], true).await?;
                sqlx::query(
                    "UPDATE orders SET submission = 'rejected', reject_reason = $2, reject_detail = $3 \
                     WHERE order_id = $1 AND submission = 'sent'",
                )
                .bind(order.order_id)
                .bind(reason as i16)
                .bind(&response.reject_detail)
                .execute(&state.pg_connections)
                .await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order() -> UnconfirmedOrder {
        UnconfirmedOrder {
            order_id: Uuid::from_u128(1),
            command_id: Uuid::from_u128(2),
            user_id: Uuid::from_u128(3),
            symbol: "BTC-USD".into(),
            base_account_id: Uuid::from_u128(4),
            quote_account_id: Uuid::from_u128(5),
            side: 1,
            order_type: 1,
            time_in_force: 2,
            base_quantity: Some("340282366920938463463374607431768211455".into()), // u128::MAX
            quote_quantity: None,
            limit_price: Some("200000".into()),
            trigger_price: None,
            trigger_direction: None,
            protection_price: None,
            expire_seconds: Some(1_900_000_000),
            expire_micros: Some(250_000),
            maker_fee_bps: 10,
            taker_fee_bps: 20,
        }
    }

    #[test]
    fn the_command_is_rebuilt_from_the_order() {
        let r = place_order_request(&order()).unwrap();
        assert_eq!(u128::from(r.command_id.unwrap()), 2);
        assert_eq!(u128::from(r.order_id.unwrap()), 1);
        assert_eq!((r.side, r.r#type, r.time_in_force), (1, 1, 2));
        assert_eq!(r.limit_price.map(u128::from), Some(200_000));
        assert_eq!(r.trigger_direction, 0);
        assert_eq!((r.maker_fee_bps, r.taker_fee_bps), (10, 20));
        let t = r.expire_time.unwrap();
        assert_eq!((t.seconds, t.nanos), (1_900_000_000, 250_000_000));
        match r.amount {
            Some(proto::place_order_request::Amount::BaseQuantity(q)) => {
                assert_eq!(u128::from(q), u128::MAX)
            }
            other => panic!("wrong amount {other:?}"),
        }
    }

    #[test]
    fn an_order_needs_exactly_one_quantity() {
        let mut both = order();
        both.quote_quantity = Some("1".into());
        assert!(place_order_request(&both).is_err());
        let mut none = order();
        none.base_quantity = None;
        assert!(place_order_request(&none).is_err());
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::settlement::test_db::{TestDb, order, seed};

    #[tokio::test]
    async fn only_old_reservations_are_stale() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let s = seed(&db.pool).await;
        let old = order(&db.pool, &s, 1, "reserving", None, None, 600).await;
        let _young = order(&db.pool, &s, 1, "reserving", None, None, 20).await;
        let _sent = order(&db.pool, &s, 1, "sent", None, None, 600).await;
        assert_eq!(stale_reservations(&db.pool).await.unwrap(), vec![old]);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn an_unconfirmed_order_gives_back_its_command() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let s = seed(&db.pool).await;
        let old = order(&db.pool, &s, 2, "sent", None, Some("150000"), 120).await;
        let _young = order(&db.pool, &s, 2, "sent", None, None, 5).await;
        let _confirmed = order(&db.pool, &s, 2, "sent", Some(1), None, 120).await;
        // the trigger that keeps updated_at current would make the order look recent
        sqlx::query("ALTER TABLE orders DISABLE TRIGGER orders_set_updated_at")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE orders SET time_in_force = 2, expire_time = timestamptz '2030-03-17 17:46:40.25+00' WHERE order_id = $1")
            .bind(old)
            .execute(&db.pool)
            .await
            .unwrap();

        let found = unconfirmed_orders(&db.pool).await.unwrap();
        assert_eq!(found.len(), 1);
        let request = place_order_request(&found[0]).unwrap();
        assert_eq!(u128::from(request.order_id.unwrap()), old.as_u128());
        assert_eq!(request.market_symbol, "BTC-USD");
        assert_eq!((request.side, request.time_in_force), (2, 2));
        assert_eq!(request.limit_price.map(u128::from), Some(150_000));
        let t = request.expire_time.unwrap();
        assert_eq!((t.seconds, t.nanos), (1_900_000_000, 250_000_000));
        db.cleanup().await;
    }
}
