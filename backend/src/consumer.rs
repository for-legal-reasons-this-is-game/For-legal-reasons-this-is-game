// The consumer of the engine's event stream. Only one pod runs it at a time (see leader.rs).
//
// It settles the trades in tigerbeetle, refunds the reservations of finished orders and writes what
// the engine reports (trades, order status, fills) to postgres. The cursor (engine_stream_state) moves
// in the same postgres transaction as those writes, and only after tigerbeetle accepted the transfers.
// A crash in between replays the batch, and the deterministic transfer ids make that harmless.
use std::collections::{HashMap, HashSet};

use num_traits::FromPrimitive;
use proto::{
    EventItem, ExecutionReport, SetMarketRequest, SubscribeEventsRequest, control, event_item::Body,
};
use sqlx::{PgConnection, PgPool};
use tokio::sync::watch;
use tokio::time::{Duration, Instant, sleep, sleep_until};
use uuid::Uuid;

use crate::domain::{CancelReason, OrderStatus};
use crate::settlement::{
    MarketInfo, Money, StreamError, load_markets, load_orders, parse_settlement, refund_orders,
    submit_money, validate_settlement,
};
use crate::v1::AppState;

const BATCH_SIZE: usize = 100; // items in a batch
const BATCH_WAIT: Duration = Duration::from_millis(50); // or this long after the first one
const RECONNECT_WAIT: Duration = Duration::from_secs(2);
// an order created this recently can still be on its way to the engine while a snapshot is made
const SNAPSHOT_GRACE_SECONDS: i32 = 60;

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub epoch: i64,
    pub sequence: i64,
    // false while an engine restart is being recovered from
    pub accepting: bool,
}

async fn load_cursor(state: &AppState) -> Result<Cursor, StreamError> {
    let (epoch, sequence, accepting) = sqlx::query_as::<_, (i64, i64, bool)>(
        "SELECT engine_epoch, last_sequence, accepting_orders FROM engine_stream_state WHERE id = 1",
    )
    .fetch_one(&state.pg_connections)
    .await?;
    Ok(Cursor {
        epoch,
        sequence,
        accepting,
    })
}

enum SessionEnd {
    Stopped,
    Reconnect(String),
    EpochChanged(i64),
}

// runs until it is told to stop (Ok) or until something that needs a human happens (Err): a
// settlement that doesn't match our orders, tigerbeetle refusing a chain, a cursor moved by someone
// else. It never skips an item and never moves the cursor past one that failed.
pub async fn run(state: AppState, mut stop: watch::Receiver<bool>) -> Result<(), StreamError> {
    loop {
        if *stop.borrow() {
            return Ok(());
        }
        let mut cursor = load_cursor(&state).await?;
        if !cursor.accepting {
            println!("CONSUMER: recovering from an engine restart");
            resume_restart(&state).await?;
        }

        match session(&state, &mut stop, &mut cursor).await? {
            SessionEnd::Stopped => return Ok(()),
            SessionEnd::Reconnect(why) => {
                eprintln!("CONSUMER: reconnecting: {why}");
                tokio::select! {
                    _ = sleep(RECONNECT_WAIT) => {}
                    _ = stop.changed() => {}
                }
            }
            SessionEnd::EpochChanged(epoch) => {
                println!("CONSUMER: the engine has a new epoch ({epoch}), it restarted");
                begin_restart(&state.pg_connections, epoch).await?;
            }
        }
    }
}

// what a session remembers while it reads the stream
struct SessionState {
    pending: Vec<EventItem>, // live items waiting to be processed together
    deadline: Option<Instant>,
    snapshot: Option<HashSet<Uuid>>, // Some while a snapshot is being received: the orders it listed
    next_expected: i64,              // the sequence the next live item has to have
    restart_pending: bool,           // this snapshot is the one that ends an engine restart
}

async fn session(
    state: &AppState,
    stop: &mut watch::Receiver<bool>,
    cursor: &mut Cursor,
) -> Result<SessionEnd, StreamError> {
    let request = SubscribeEventsRequest {
        after_sequence: cursor.sequence as u64,
        engine_epoch: cursor.epoch as u64,
    };
    let mut stream = match state.engine_client.subscribe_events(request).await {
        Ok(stream) => stream,
        Err(status) => {
            return Ok(SessionEnd::Reconnect(format!(
                "couldn't subscribe: {status}"
            )));
        }
    };

    let mut s = SessionState {
        pending: Vec::new(),
        deadline: None,
        snapshot: None,
        next_expected: cursor.sequence + 1,
        restart_pending: !cursor.accepting,
    };

    loop {
        let deadline = s.deadline;
        let batch_timer = async move {
            match deadline {
                Some(d) => sleep_until(d).await,
                None => std::future::pending::<()>().await,
            }
        };

        tokio::select! {
            _ = stop.changed() => {
                flush(state, cursor, &mut s).await?;
                return Ok(SessionEnd::Stopped);
            }
            _ = batch_timer => flush(state, cursor, &mut s).await?,
            message = stream.message() => match message {
                Ok(Some(item)) => {
                    if let Some(end) = handle_item(state, cursor, &mut s, item).await? {
                        return Ok(end);
                    }
                }
                Ok(None) => {
                    flush(state, cursor, &mut s).await?;
                    return Ok(SessionEnd::Reconnect("the engine closed the stream".into()));
                }
                Err(status) => {
                    flush(state, cursor, &mut s).await?;
                    return Ok(SessionEnd::Reconnect(format!("stream error: {status}")));
                }
            },
        }
    }
}

async fn handle_item(
    state: &AppState,
    cursor: &mut Cursor,
    s: &mut SessionState,
    item: EventItem,
) -> Result<Option<SessionEnd>, StreamError> {
    // a different epoch means the engine restarted, and what we believe about it is gone
    if item.engine_epoch as i64 != cursor.epoch {
        flush(state, cursor, s).await?;
        return Ok(Some(SessionEnd::EpochChanged(item.engine_epoch as i64)));
    }

    match &item.body {
        Some(Body::Control(c)) => match control::Kind::try_from(c.kind) {
            Ok(control::Kind::SnapshotBegin) => {
                flush(state, cursor, s).await?;
                println!("CONSUMER: snapshot begins");
                s.snapshot = Some(HashSet::new());
            }
            Ok(control::Kind::SnapshotEnd) => {
                flush(state, cursor, s).await?;
                let seen = s.snapshot.take().unwrap_or_default();
                finish_snapshot(
                    state,
                    cursor,
                    &seen,
                    c.as_of_sequence as i64,
                    s.restart_pending,
                )
                .await?;
                s.next_expected = cursor.sequence + 1;
                s.restart_pending = false;
                println!(
                    "CONSUMER: snapshot done, the cursor is at {}",
                    cursor.sequence
                );
            }
            _ => eprintln!(
                "CONSUMER: control message of an unknown kind {}, skipped",
                c.kind
            ),
        },

        // a RESTATED report is a line of a snapshot: it only copies the state of an order
        Some(Body::ExecutionReport(r)) if r.exec_type == proto::ExecType::Restated as i32 => {
            match s.snapshot.as_mut() {
                Some(seen) => apply_restated(state, r, seen).await?,
                None => eprintln!("CONSUMER: a RESTATED report outside of a snapshot, skipped"),
            }
        }

        _ => {
            if s.snapshot.is_some() {
                return Ok(Some(SessionEnd::Reconnect(
                    "a live item arrived in the middle of a snapshot".into(),
                )));
            }
            let sequence = item.sequence as i64;
            if sequence < s.next_expected {
                return Ok(None); // processed already, normal after a reconnect
            }
            if sequence > s.next_expected {
                flush(state, cursor, s).await?;
                return Ok(Some(SessionEnd::Reconnect(format!(
                    "GAP in the event stream: expected {}, got {sequence}",
                    s.next_expected
                ))));
            }
            s.next_expected += 1;
            s.pending.push(item);
            if s.deadline.is_none() {
                s.deadline = Some(Instant::now() + BATCH_WAIT);
            }
            if s.pending.len() >= BATCH_SIZE {
                flush(state, cursor, s).await?;
            }
        }
    }
    Ok(None)
}

async fn flush(
    state: &AppState,
    cursor: &mut Cursor,
    s: &mut SessionState,
) -> Result<(), StreamError> {
    s.deadline = None;
    if s.pending.is_empty() {
        return Ok(());
    }
    let items = std::mem::take(&mut s.pending);
    commit_batch(state, cursor, items).await
}

// what a live item does to the database
enum Action {
    Trade(TradeRecord),
    Report(ReportRecord),
    Nothing,
}

struct TradeRecord {
    trade_id: Uuid,
    market_id: i32,
    buyer_order: Uuid,
    seller_order: Uuid,
    price: u128,
    base_quantity: u128,
    quote_quantity: u128,
    buyer_fee: u128,
    seller_fee: u128,
    taker_side: i16,
    epoch: i64,
    sequence: i64,
    time_seconds: Option<i64>,
    time_micros: i64,
}

struct ReportRecord {
    order_id: Uuid,
    status: OrderStatus,
    cancel_reason: Option<CancelReason>,
    filled_base: u128,
    filled_quote: u128,
    limit_price: Option<u128>,
}

impl ReportRecord {
    // the reservation of a finished order goes back to the user
    fn is_final(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Expired
        )
    }
}

struct Prepared {
    sequence: i64,
    action: Action,
    money: Option<Money>, // what the item moves in tigerbeetle, None if it moves nothing
}

fn report_record(r: &ExecutionReport) -> Result<ReportRecord, String> {
    let order_id = r
        .order_id
        .map(|i| Uuid::from_u128(u128::from(i)))
        .ok_or("no order_id")?;
    let status = OrderStatus::from_i32(r.status).ok_or(format!("unknown status {}", r.status))?;
    Ok(ReportRecord {
        order_id,
        status,
        cancel_reason: CancelReason::from_i32(r.cancel_reason), // 0 is "none"
        filled_base: r
            .filled_base_quantity
            .map(u128::from)
            .ok_or("no filled_base_quantity")?,
        filled_quote: r
            .filled_quote_quantity
            .map(u128::from)
            .ok_or("no filled_quote_quantity")?,
        limit_price: r.limit_price.map(u128::from),
    })
}

// turns the live items into what has to happen. A settlement that doesn't match our data is an
// error that stops everything, a report we can't use is logged and skipped.
async fn prepare_batch(
    state: &AppState,
    items: &[EventItem],
) -> Result<Vec<Prepared>, StreamError> {
    let mut ids: Vec<Uuid> = Vec::new();
    for item in items {
        match &item.body {
            Some(Body::TradeSettlement(t)) => ids.extend(
                [t.buyer_order_id, t.seller_order_id]
                    .into_iter()
                    .flatten()
                    .map(|i| Uuid::from_u128(u128::from(i))),
            ),
            Some(Body::ExecutionReport(r)) => {
                ids.extend(r.order_id.map(|i| Uuid::from_u128(u128::from(i))))
            }
            _ => {}
        }
    }
    let orders = load_orders(&state.pg_connections, &ids).await?;
    let markets = load_markets(&state.pg_connections).await?;
    let by_symbol: HashMap<&str, &MarketInfo> =
        markets.iter().map(|m| (m.symbol.as_str(), m)).collect();
    let by_id: HashMap<i32, &MarketInfo> = markets.iter().map(|m| (m.market_id, m)).collect();

    let mut prepared = Vec::with_capacity(items.len());
    for item in items {
        let sequence = item.sequence as i64;
        let (action, money) = match &item.body {
            Some(Body::TradeSettlement(t)) => {
                let fail = |why: String| format!("settlement at sequence {sequence}: {why}");
                let p = parse_settlement(t).map_err(fail)?;
                let market = *by_symbol
                    .get(item.market_symbol.as_str())
                    .ok_or_else(|| fail(format!("unknown market {}", item.market_symbol)))?;
                let buyer_id = Uuid::from_u128(p.buyer_order);
                let seller_id = Uuid::from_u128(p.seller_order);
                let buyer = orders
                    .get(&buyer_id)
                    .ok_or_else(|| fail(format!("unknown order {buyer_id}")))?;
                let seller = orders
                    .get(&seller_id)
                    .ok_or_else(|| fail(format!("unknown order {seller_id}")))?;
                validate_settlement(&p, buyer, seller, market).map_err(fail)?;

                let time = item.engine_time.as_ref();
                let record = TradeRecord {
                    trade_id: Uuid::from_u128(p.trade_id),
                    market_id: market.market_id,
                    buyer_order: buyer_id,
                    seller_order: seller_id,
                    price: p.price,
                    base_quantity: p.base_quantity,
                    quote_quantity: p.quote_quantity,
                    buyer_fee: p.buyer_fee,
                    seller_fee: p.seller_fee,
                    taker_side: p.taker_side as i16,
                    epoch: item.engine_epoch as i64,
                    sequence,
                    time_seconds: time.map(|t| t.seconds),
                    time_micros: time.map(|t| i64::from(t.nanos) / 1000).unwrap_or(0),
                };
                (Action::Trade(record), Some(Money::Trade(p, market.clone())))
            }

            Some(Body::ExecutionReport(r)) => match report_record(r) {
                Err(why) => {
                    eprintln!(
                        "CONSUMER: sequence {sequence}: unusable execution report, skipped: {why}"
                    );
                    (Action::Nothing, None)
                }
                Ok(record) => match orders.get(&record.order_id) {
                    None => {
                        eprintln!(
                            "CONSUMER: sequence {sequence}: report of unknown order {}, skipped",
                            record.order_id
                        );
                        (Action::Nothing, None)
                    }
                    Some(order) => {
                        let market = by_id.get(&order.market_id).ok_or_else(|| {
                            format!("order {} has an unknown market", order.order_id)
                        })?;
                        let money = record
                            .is_final()
                            .then(|| Money::Refund(order.clone(), (*market).clone()));
                        (Action::Report(record), money)
                    }
                },
            },

            _ => {
                eprintln!("CONSUMER: sequence {sequence}: item of an unknown kind, skipped");
                (Action::Nothing, None)
            }
        };
        prepared.push(Prepared {
            sequence,
            action,
            money,
        });
    }
    Ok(prepared)
}

// settles and records a batch of live items. If tigerbeetle refuses an item, the ones before it are
// still recorded and the error stops the consumer.
async fn commit_batch(
    state: &AppState,
    cursor: &mut Cursor,
    items: Vec<EventItem>,
) -> Result<(), StreamError> {
    let prepared = prepare_batch(state, &items).await?;

    let events: Vec<Option<Money>> = prepared.iter().map(|p| p.money.clone()).collect();
    let failure = submit_money(&state.tb_client, &events, false).await?;
    let applicable = failure.as_ref().map_or(prepared.len(), |f| f.group);

    if applicable > 0 {
        let last = prepared[applicable - 1].sequence;
        let mut tx = state.pg_connections.begin().await?;
        for p in &prepared[..applicable] {
            match &p.action {
                Action::Trade(t) => insert_trade(&mut tx, t).await?,
                Action::Report(r) => update_order(&mut *tx, r).await?,
                Action::Nothing => {}
            }
        }
        advance_cursor(&mut tx, cursor, last).await?;
        tx.commit().await?;
        cursor.sequence = last;
    }

    if let Some(f) = failure {
        return Err(format!(
            "tigerbeetle refused the transfers of sequence {}: {}",
            prepared[f.group].sequence,
            f.detail()
        )
        .into());
    }
    Ok(())
}

// the cursor only moves if nobody else moved it: it's what stops a consumer that lost the lock
async fn advance_cursor(
    conn: &mut PgConnection,
    cursor: &Cursor,
    to: i64,
) -> Result<(), StreamError> {
    let moved = sqlx::query(
        "UPDATE engine_stream_state SET last_sequence = $1, updated_at = now() \
         WHERE id = 1 AND engine_epoch = $2 AND last_sequence = $3",
    )
    .bind(to)
    .bind(cursor.epoch)
    .bind(cursor.sequence)
    .execute(conn)
    .await?;
    if moved.rows_affected() != 1 {
        return Err("the cursor was moved by someone else, another consumer is active".into());
    }
    Ok(())
}

async fn insert_trade(conn: &mut PgConnection, t: &TradeRecord) -> Result<(), sqlx::Error> {
    // amounts go as text because a u128 doesn't always fit a Decimal
    sqlx::query(
        "INSERT INTO trades (trade_id, market_id, buyer_order_id, seller_order_id, price, \
             base_quantity, quote_quantity, buyer_fee, seller_fee, taker_side, engine_epoch, \
             engine_sequence, engine_time) \
         VALUES ($1, $2, $3, $4, $5::text::numeric, $6::text::numeric, $7::text::numeric, \
             $8::text::numeric, $9::text::numeric, $10, $11, $12, \
             COALESCE(timestamptz 'epoch' + $13::bigint * interval '1 second' \
                      + $14::bigint * interval '1 microsecond', now())) \
         ON CONFLICT (trade_id) DO NOTHING",
    )
    .bind(t.trade_id)
    .bind(t.market_id)
    .bind(t.buyer_order)
    .bind(t.seller_order)
    .bind(t.price.to_string())
    .bind(t.base_quantity.to_string())
    .bind(t.quote_quantity.to_string())
    .bind(t.buyer_fee.to_string())
    .bind(t.seller_fee.to_string())
    .bind(t.taker_side)
    .bind(t.epoch)
    .bind(t.sequence)
    .bind(t.time_seconds)
    .bind(t.time_micros)
    .execute(conn)
    .await?;
    Ok(())
}

// a report has the whole current state of the order, so it overwrites, it never adds
async fn update_order<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    r: &ReportRecord,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE orders SET status = $2, cancel_reason = $3, \
             filled_base_quantity = $4::text::numeric, filled_quote_quantity = $5::text::numeric, \
             limit_price = COALESCE($6::text::numeric, limit_price) \
         WHERE order_id = $1",
    )
    .bind(r.order_id)
    .bind(r.status as i16)
    .bind(r.cancel_reason.map(|c| c as i16))
    .bind(r.filled_base.to_string())
    .bind(r.filled_quote.to_string())
    .bind(r.limit_price.map(|p| p.to_string()))
    .execute(executor)
    .await?;
    Ok(())
}

// a line of a snapshot: copies the state of a working order, and remembers that the engine has it
async fn apply_restated(
    state: &AppState,
    r: &ExecutionReport,
    seen: &mut HashSet<Uuid>,
) -> Result<(), StreamError> {
    match report_record(r) {
        Ok(record) => {
            update_order(&state.pg_connections, &record).await?;
            seen.insert(record.order_id);
        }
        Err(why) => eprintln!("CONSUMER: unusable RESTATED report, skipped: {why}"),
    }
    Ok(())
}

// SNAPSHOT_END: every order we consider working that the snapshot didn't list is gone from the
// engine, so it's cancelled here and its reservation refunded. Then the cursor goes to the snapshot's.
async fn finish_snapshot(
    state: &AppState,
    cursor: &mut Cursor,
    seen: &HashSet<Uuid>,
    as_of_sequence: i64,
    restart_pending: bool,
) -> Result<(), StreamError> {
    if !restart_pending {
        eprintln!(
            "CONSUMER: !! the engine sent a snapshot we didn't ask for. Settlements missed in the \
             gap can't be recovered; the orders missing from the snapshot are cancelled"
        );
    }

    let missing = snapshot_missing(&state.pg_connections, seen, restart_pending).await?;
    if !missing.is_empty() {
        println!(
            "CONSUMER: {} orders are not in the snapshot, cancelling them",
            missing.len()
        );
    }
    refund_orders(state, &missing, true).await?;
    apply_snapshot_end(
        &state.pg_connections,
        cursor.epoch,
        &missing,
        as_of_sequence,
    )
    .await?;

    cursor.sequence = as_of_sequence;
    cursor.accepting = true;
    Ok(())
}

// Working orders: the ones the backend accepted, and the ones the stream reported. An order that was
// sent but is still unconfirmed is left alone (the recovery job and the restart procedure own it).
// Outside of a restart, orders from the last minute can still be on their way to the engine.
async fn snapshot_missing(
    pool: &PgPool,
    seen: &HashSet<Uuid>,
    restart_pending: bool,
) -> Result<Vec<Uuid>, sqlx::Error> {
    let candidates = sqlx::query_scalar::<_, Uuid>(
        "SELECT order_id FROM orders \
         WHERE ((submission = 'accepted' AND (status IS NULL OR status IN (1, 2, 6))) \
                OR (submission = 'sent' AND status IN (1, 2, 6))) \
           AND ($1 OR created_at < now() - $2::int * interval '1 second')",
    )
    .bind(restart_pending)
    .bind(SNAPSHOT_GRACE_SECONDS)
    .fetch_all(pool)
    .await?;
    Ok(candidates
        .into_iter()
        .filter(|id| !seen.contains(id))
        .collect())
}

// cancels the missing orders and moves the cursor to the snapshot's, in one transaction
async fn apply_snapshot_end(
    pool: &PgPool,
    epoch: i64,
    missing: &[Uuid],
    as_of_sequence: i64,
) -> Result<(), StreamError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE orders SET status = $2, cancel_reason = $3 \
         WHERE order_id = ANY($1) AND (status IS NULL OR status IN (1, 2, 6))",
    )
    .bind(missing)
    .bind(OrderStatus::Cancelled as i16)
    .bind(CancelReason::Admin as i16)
    .execute(&mut *tx)
    .await?;
    let moved = sqlx::query(
        "UPDATE engine_stream_state SET last_sequence = $1, accepting_orders = TRUE, updated_at = now() \
         WHERE id = 1 AND engine_epoch = $2",
    )
    .bind(as_of_sequence)
    .bind(epoch)
    .execute(&mut *tx)
    .await?;
    if moved.rows_affected() != 1 {
        return Err("the stream state changed while the snapshot was processed".into());
    }
    tx.commit().await?;
    Ok(())
}

// orders the engine never confirmed: with a restarted engine their commands are lost
async fn in_flight_orders(pool: &PgPool) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT order_id FROM orders WHERE submission = 'sent' AND status IS NULL",
    )
    .fetch_all(pool)
    .await
}

async fn reject_in_flight(pool: &PgPool, ids: &[Uuid]) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE orders SET submission = 'rejected', \
             reject_detail = 'the engine restarted before it confirmed the order' \
         WHERE order_id = ANY($1) AND submission = 'sent' AND status IS NULL",
    )
    .bind(ids)
    .execute(pool)
    .await?;
    Ok(())
}

// Engine restart, step 1: remember the new epoch and stop every pod from placing orders.
// The cursor goes to 0 so the next subscription asks for a snapshot.
async fn begin_restart(pool: &PgPool, epoch: i64) -> Result<(), StreamError> {
    sqlx::query(
        "UPDATE engine_stream_state \
         SET engine_epoch = $1, last_sequence = 0, accepting_orders = FALSE, updated_at = now() \
         WHERE id = 1",
    )
    .bind(epoch)
    .execute(pool)
    .await?;
    Ok(())
}

// Engine restart, the rest: commands that were in flight are lost with the old engine, so they are
// refunded and rejected; then the engine gets its markets again. It can be repeated safely, so a
// consumer that dies in the middle is replaced by one that starts over. Accepting orders comes back
// when the snapshot of the new engine has been processed.
async fn resume_restart(state: &AppState) -> Result<(), StreamError> {
    let in_flight = in_flight_orders(&state.pg_connections).await?;
    if !in_flight.is_empty() {
        println!(
            "CONSUMER: refunding {} orders that were in flight",
            in_flight.len()
        );
    }
    refund_orders(state, &in_flight, true).await?;
    reject_in_flight(&state.pg_connections, &in_flight).await?;

    let markets = sqlx::query_as::<_, (String, i16, i16, i16, Uuid)>(
        "SELECT m.symbol, bl.decimals, ql.decimals, m.price_decimals, m.fee_account_id \
         FROM markets m \
         JOIN ledgers bl ON bl.ledger_id = m.base_ledger_id \
         JOIN ledgers ql ON ql.ledger_id = m.quote_ledger_id \
         ORDER BY m.market_id",
    )
    .fetch_all(&state.pg_connections)
    .await?;
    for (symbol, base_decimals, quote_decimals, price_decimals, fee_account) in markets {
        let response = state
            .engine_client
            .set_market(SetMarketRequest {
                market: Some(proto::Market {
                    symbol: symbol.clone(),
                    base_decimals: base_decimals as u32,
                    quote_decimals: quote_decimals as u32,
                    price_decimals: price_decimals as u32,
                    fee_account: Some(fee_account.as_u128().into()),
                }),
            })
            .await
            .map_err(|status| format!("SetMarket({symbol}) failed: {status}"))?;
        if response.reject_reason != 0 {
            return Err(format!(
                "the engine refused SetMarket({symbol}): {} ({})",
                response.reject_reason, response.reject_detail
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Side;
    use crate::settlement::test_db::{TestDb, order, seed};
    use crate::settlement::{ParsedSettlement, load_markets, load_orders, validate_settlement};

    #[tokio::test]
    async fn trades_and_reports_are_written() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let s = seed(&db.pool).await;
        let buy = order(&db.pool, &s, 1, "accepted", None, Some("200000"), 5).await;
        let sell = order(&db.pool, &s, 2, "accepted", None, None, 5).await;

        // the rows of the orders and of the market are what the settlement is checked against
        let orders = load_orders(&db.pool, &[buy, sell]).await.unwrap();
        let market = load_markets(&db.pool).await.unwrap().remove(0);
        assert_eq!(
            (orders[&buy].side, orders[&sell].side),
            (Side::Buy, Side::Sell)
        );
        assert_eq!(orders[&buy].limit_price.as_deref(), Some("200000"));
        let parsed = ParsedSettlement {
            trade_id: 77,
            price: 200_000,
            base_quantity: 5,
            quote_quantity: 1_000_000,
            taker_side: Side::Buy,
            buyer_order: buy.as_u128(),
            buyer_holder: s.buyer.user.as_u128(),
            buyer_base_account: s.buyer.base.as_u128(),
            buyer_quote_account: s.buyer.quote.as_u128(),
            buyer_fee: 2_000,
            seller_order: sell.as_u128(),
            seller_holder: s.seller.user.as_u128(),
            seller_base_account: s.seller.base.as_u128(),
            seller_quote_account: s.seller.quote.as_u128(),
            seller_fee: 1_000,
            fee_account: s.fee_account.as_u128(),
        };
        assert_eq!(
            validate_settlement(&parsed, &orders[&buy], &orders[&sell], &market),
            Ok(())
        );

        // the trade, twice: the second is ignored
        let record = TradeRecord {
            trade_id: Uuid::from_u128(77),
            market_id: s.market_id,
            buyer_order: buy,
            seller_order: sell,
            price: 200_000,
            base_quantity: 5,
            quote_quantity: u128::MAX,
            buyer_fee: 2_000,
            seller_fee: 1_000,
            taker_side: 1,
            epoch: 99,
            sequence: 3,
            time_seconds: Some(1_900_000_000),
            time_micros: 250_000,
        };
        let mut conn = db.pool.acquire().await.unwrap();
        insert_trade(&mut conn, &record).await.unwrap();
        insert_trade(&mut conn, &record).await.unwrap();
        let (count, quote, time): (i64, String, String) = sqlx::query_as(
            "SELECT count(*), max(quote_quantity)::text, \
                    max(to_char(engine_time AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US')) FROM trades",
        )
        .fetch_one(&db.pool).await.unwrap();
        assert_eq!(count, 1);
        assert_eq!(quote, u128::MAX.to_string()); // doesn't fit a Decimal, fits NUMERIC(39,0)
        assert_eq!(time, "2030-03-17T17:46:40.250000");

        // a report overwrites the totals, and without a limit price it keeps the one it had
        let report = ReportRecord {
            order_id: buy,
            status: OrderStatus::PartiallyFilled,
            cancel_reason: None,
            filled_base: 2,
            filled_quote: u128::MAX,
            limit_price: None,
        };
        update_order(&db.pool, &report).await.unwrap();
        let (status, filled, limit): (i16, String, String) = sqlx::query_as(
            "SELECT status, filled_quote_quantity::text, limit_price::text FROM orders WHERE order_id = $1",
        )
        .bind(buy).fetch_one(&db.pool).await.unwrap();
        assert_eq!(
            (status, filled, limit),
            (2, u128::MAX.to_string(), "200000".to_string())
        );

        let report = ReportRecord {
            order_id: buy,
            status: OrderStatus::Cancelled,
            cancel_reason: Some(CancelReason::UserRequest),
            filled_base: 2,
            filled_quote: 400_000,
            limit_price: Some(199_000),
        };
        update_order(&db.pool, &report).await.unwrap();
        let (status, reason, limit): (i16, Option<i16>, String) = sqlx::query_as(
            "SELECT status, cancel_reason, limit_price::text FROM orders WHERE order_id = $1",
        )
        .bind(buy)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!((status, reason, limit), (4, Some(1), "199000".to_string()));

        drop(conn); // closing the pool waits for every connection to come back
        db.cleanup().await;
    }

    #[tokio::test]
    async fn the_cursor_only_moves_for_who_holds_it() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        begin_restart(&db.pool, 500).await.unwrap();
        let cursor = load_cursor_from(&db.pool).await;
        assert_eq!(
            (cursor.epoch, cursor.sequence, cursor.accepting),
            (500, 0, false)
        );

        let mut conn = db.pool.acquire().await.unwrap();
        advance_cursor(&mut conn, &cursor, 10).await.unwrap();
        // the same position again is a consumer that lost the lock
        assert!(advance_cursor(&mut conn, &cursor, 11).await.is_err());
        let stale_epoch = Cursor {
            epoch: 499,
            sequence: 10,
            accepting: false,
        };
        assert!(advance_cursor(&mut conn, &stale_epoch, 11).await.is_err());
        assert_eq!(load_cursor_from(&db.pool).await.sequence, 10);
        drop(conn);
        db.cleanup().await;
    }

    async fn load_cursor_from(pool: &PgPool) -> Cursor {
        let (epoch, sequence, accepting) = sqlx::query_as::<_, (i64, i64, bool)>(
            "SELECT engine_epoch, last_sequence, accepting_orders FROM engine_stream_state WHERE id = 1",
        )
        .fetch_one(pool).await.unwrap();
        Cursor {
            epoch,
            sequence,
            accepting,
        }
    }

    #[tokio::test]
    async fn a_snapshot_cancels_the_orders_the_engine_doesnt_have() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let s = seed(&db.pool).await;
        let p = &db.pool;
        let in_snapshot = order(p, &s, 1, "accepted", Some(1), None, 300).await;
        let gone = order(p, &s, 1, "accepted", Some(2), None, 300).await;
        let gone_unreported = order(p, &s, 2, "accepted", None, None, 300).await;
        let gone_sent = order(p, &s, 2, "sent", Some(1), None, 300).await;
        let in_flight = order(p, &s, 1, "sent", None, None, 300).await; // unconfirmed: not the snapshot's business
        let finished = order(p, &s, 1, "accepted", Some(3), None, 300).await;
        let rejected = order(p, &s, 1, "rejected", None, None, 300).await;
        let recent = order(p, &s, 1, "accepted", Some(1), None, 5).await;

        let seen: HashSet<Uuid> = [in_snapshot].into();
        let mut missing = snapshot_missing(p, &seen, false).await.unwrap();
        missing.sort();
        let mut expected = vec![gone, gone_unreported, gone_sent];
        expected.sort();
        assert_eq!(missing, expected); // `recent` can still be on its way to the engine

        // after a restart the engine is empty by definition, so there is no grace period
        let mut after_restart = snapshot_missing(p, &seen, true).await.unwrap();
        after_restart.sort();
        let mut expected = vec![gone, gone_unreported, gone_sent, recent];
        expected.sort();
        assert_eq!(after_restart, expected);
        assert!(
            !after_restart.contains(&in_flight)
                && !after_restart.contains(&finished)
                && !after_restart.contains(&rejected)
        );

        // ending the snapshot cancels them and moves the cursor, and the orders open again
        begin_restart(p, 700).await.unwrap();
        apply_snapshot_end(p, 700, &after_restart, 42)
            .await
            .unwrap();
        let (status, reason): (i16, Option<i16>) =
            sqlx::query_as("SELECT status, cancel_reason FROM orders WHERE order_id = $1")
                .bind(gone)
                .fetch_one(p)
                .await
                .unwrap();
        assert_eq!((status, reason), (4, Some(7)));
        let untouched: Option<i16> =
            sqlx::query_scalar("SELECT status FROM orders WHERE order_id = $1")
                .bind(in_snapshot)
                .fetch_one(p)
                .await
                .unwrap();
        assert_eq!(untouched, Some(1));
        let cursor = load_cursor_from(p).await;
        assert_eq!(
            (cursor.epoch, cursor.sequence, cursor.accepting),
            (700, 42, true)
        );

        // a snapshot of an epoch that isn't the stored one is refused and changes nothing
        assert!(
            apply_snapshot_end(p, 701, &[in_snapshot], 50)
                .await
                .is_err()
        );
        assert_eq!(load_cursor_from(p).await.sequence, 42);
        db.cleanup().await;
    }

    #[tokio::test]
    async fn orders_in_flight_are_rejected_on_a_restart() {
        let Some(db) = TestDb::new().await else {
            return;
        };
        let s = seed(&db.pool).await;
        let p = &db.pool;
        let in_flight = order(p, &s, 1, "sent", None, None, 10).await;
        let confirmed = order(p, &s, 1, "sent", Some(1), None, 10).await;
        let accepted = order(p, &s, 1, "accepted", None, None, 10).await;

        let found = in_flight_orders(p).await.unwrap();
        assert_eq!(found, vec![in_flight]);
        reject_in_flight(p, &found).await.unwrap();

        let submission = |id: Uuid| {
            let pool = db.pool.clone();
            async move {
                sqlx::query_scalar::<_, String>(
                    "SELECT submission::text FROM orders WHERE order_id = $1",
                )
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };
        assert_eq!(submission(in_flight).await, "rejected");
        assert_eq!(submission(confirmed).await, "sent");
        assert_eq!(submission(accepted).await, "accepted");
        db.cleanup().await;
    }

    // ---- the whole consumer against a fake engine (a real gRPC server), postgres and tigerbeetle ----
    use cn_tigerbeetle as tb;
    use proto::trading_engine_server::{TradingEngine, TradingEngineServer};
    use std::sync::{Arc, Mutex};
    use tokio::sync::mpsc;
    use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
    use tonic::{Request, Response, Status};

    const EPOCH: u64 = 1_234_567_890_123;

    type EventSender = mpsc::Sender<Result<EventItem, Status>>;
    type Scripts = HashMap<(u64, u64), Vec<EventItem>>;

    // sends the script of the (epoch, after_sequence) it is subscribed with and keeps the stream open
    #[derive(Clone, Default)]
    struct FakeEngine {
        scripts: Arc<Mutex<Scripts>>,
        subscriptions: Arc<Mutex<Vec<(u64, u64)>>>,
        markets: Arc<Mutex<Vec<String>>>,
        open_streams: Arc<Mutex<Vec<EventSender>>>,
    }

    #[tonic::async_trait]
    impl TradingEngine for FakeEngine {
        async fn set_market(
            &self,
            request: Request<proto::SetMarketRequest>,
        ) -> Result<Response<proto::SetMarketResponse>, Status> {
            let market = request.into_inner().market.expect("a market");
            self.markets.lock().unwrap().push(market.symbol);
            Ok(Response::new(proto::SetMarketResponse::default()))
        }
        async fn place_order(
            &self,
            _: Request<proto::PlaceOrderRequest>,
        ) -> Result<Response<proto::PlaceOrderResponse>, Status> {
            Err(Status::unimplemented("not in this test"))
        }
        async fn cancel_order(
            &self,
            _: Request<proto::CancelOrderRequest>,
        ) -> Result<Response<proto::CancelOrderResponse>, Status> {
            Err(Status::unimplemented("not in this test"))
        }
        async fn cancel_orders(
            &self,
            _: Request<proto::CancelOrdersRequest>,
        ) -> Result<Response<proto::CancelOrdersResponse>, Status> {
            Err(Status::unimplemented("not in this test"))
        }
        async fn modify_order(
            &self,
            _: Request<proto::ModifyOrderRequest>,
        ) -> Result<Response<proto::ModifyOrderResponse>, Status> {
            Err(Status::unimplemented("not in this test"))
        }
        type SubscribeEventsStream = ReceiverStream<Result<EventItem, Status>>;
        async fn subscribe_events(
            &self,
            request: Request<proto::SubscribeEventsRequest>,
        ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
            let request = request.into_inner();
            let key = (request.engine_epoch, request.after_sequence);
            self.subscriptions.lock().unwrap().push(key);
            let script = self
                .scripts
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .unwrap_or_default();
            let (sender, receiver) = mpsc::channel(64);
            for item in script {
                sender.send(Ok(item)).await.unwrap();
            }
            self.open_streams.lock().unwrap().push(sender);
            Ok(Response::new(ReceiverStream::new(receiver)))
        }
        type SubscribeMarketDataStream = ReceiverStream<Result<proto::MarketDataItem, Status>>;
        async fn subscribe_market_data(
            &self,
            _: Request<proto::SubscribeMarketDataRequest>,
        ) -> Result<Response<Self::SubscribeMarketDataStream>, Status> {
            Err(Status::unimplemented("not in this test"))
        }
    }

    fn item(sequence: u64, symbol: &str, body: Body) -> EventItem {
        EventItem {
            sequence,
            engine_epoch: EPOCH,
            engine_time: Some(proto::Timestamp {
                seconds: 1_900_000_000,
                nanos: 0,
            }),
            market_symbol: symbol.into(),
            body: Some(body),
        }
    }

    fn control_item(kind: control::Kind, as_of_sequence: u64) -> EventItem {
        item(
            0,
            "",
            Body::Control(proto::Control {
                kind: kind as i32,
                as_of_sequence,
            }),
        )
    }

    fn report(
        order: Uuid,
        exec_type: proto::ExecType,
        status: OrderStatus,
        filled: (u128, u128),
        trade: Option<u128>,
    ) -> ExecutionReport {
        ExecutionReport {
            order_id: Some(order.as_u128().into()),
            exec_type: exec_type as i32,
            status: status as i32,
            filled_base_quantity: Some(filled.0.into()),
            filled_quote_quantity: Some(filled.1.into()),
            trade_id: trade.map(Into::into),
            ..Default::default()
        }
    }

    async fn tb_account(client: &tb::Client, id: Uuid, ledger: u32, can_go_negative: bool) {
        let results = client
            .create_accounts(&[tb::Account {
                id: id.as_u128(),
                ledger,
                code: 1,
                flags: if can_go_negative {
                    tb::AccountFlags::empty()
                } else {
                    tb::AccountFlags::DebitsMustNotExceedCredits
                },
                ..Default::default()
            }])
            .await
            .unwrap();
        assert!(results.is_empty(), "{results:?}");
    }

    async fn tb_move(client: &tb::Client, from: Uuid, to: Uuid, amount: u128, ledger: u32) {
        let results = client
            .create_transfers(&[tb::Transfer {
                id: tb::id(),
                debit_account_id: from.as_u128(),
                credit_account_id: to.as_u128(),
                amount,
                ledger,
                code: 1,
                ..Default::default()
            }])
            .await
            .unwrap();
        assert!(results.is_empty(), "{results:?}");
    }

    async fn tb_balance(client: &tb::Client, id: Uuid) -> (u128, u128) {
        let account = client.lookup_accounts(&[id.as_u128()]).await.unwrap()[0];
        (account.credits_posted, account.debits_posted)
    }

    #[tokio::test]
    async fn the_consumer_recovers_from_an_engine_restart_and_settles_a_trade() {
        let (Some(db), Ok(tb_address)) = (TestDb::new().await, std::env::var("TEST_TB_ADDRESS"))
        else {
            return;
        };
        let s = seed(&db.pool).await;
        let p = &db.pool;
        let buy = order(p, &s, 1, "accepted", None, None, 30).await;
        let sell = order(p, &s, 2, "accepted", None, None, 30).await;
        let gone = order(p, &s, 1, "accepted", Some(1), None, 300).await; // the new engine doesn't have it
        let in_flight = order(p, &s, 1, "sent", None, None, 30).await; // its command died with the old engine

        // the money in tigerbeetle: ledger 1 is USD (quote), 3 is BTC (base)
        let tb_client = Arc::new(tb::Client::new(0, &tb_address).expect("connect to tigerbeetle"));
        let (source_quote, source_base) = (Uuid::new_v4(), Uuid::new_v4());
        tb_account(&tb_client, source_quote, 1, true).await;
        tb_account(&tb_client, source_base, 3, true).await;
        for (id, ledger) in [
            (s.buyer.base, 3),
            (s.buyer.quote, 1),
            (s.seller.base, 3),
            (s.seller.quote, 1),
            (s.fee_account, 1),
            (buy, 1),
            (sell, 3),
            (in_flight, 1),
        ] {
            tb_account(&tb_client, id, ledger, false).await;
        }
        tb_move(&tb_client, source_quote, s.buyer.quote, 5_000_000, 1).await;
        tb_move(&tb_client, s.buyer.quote, buy, 1_003_000, 1).await; // 1_000_000 and a 3_000 allowance for the fee
        tb_move(&tb_client, s.buyer.quote, in_flight, 500, 1).await;
        tb_move(&tb_client, source_base, s.seller.base, 50, 3).await;
        tb_move(&tb_client, s.seller.base, sell, 5, 3).await;

        // what the engine says: nothing the first time (it has a new epoch), then its snapshot (it knows
        // the two orders, not `gone`) and a trade between them that fills both
        let trade_id = tb::id();
        let settlement = proto::TradeSettlement {
            trade_id: Some(trade_id.into()),
            price: Some(200_000u128.into()),
            base_quantity: Some(5u128.into()),
            quote_quantity: Some(1_000_000u128.into()),
            taker_side: proto::Side::Buy as i32,
            buyer_order_id: Some(buy.as_u128().into()),
            buyer_account_holder: Some(s.buyer.user.as_u128().into()),
            buyer_base_account: Some(s.buyer.base.as_u128().into()),
            buyer_quote_account: Some(s.buyer.quote.as_u128().into()),
            buyer_fee: Some(2_000u128.into()),
            seller_order_id: Some(sell.as_u128().into()),
            seller_account_holder: Some(s.seller.user.as_u128().into()),
            seller_base_account: Some(s.seller.base.as_u128().into()),
            seller_quote_account: Some(s.seller.quote.as_u128().into()),
            seller_fee: Some(1_000u128.into()),
            fee_account: Some(s.fee_account.as_u128().into()),
        };
        let fake = FakeEngine::default();
        fake.scripts.lock().unwrap().insert(
            (0, 0),
            vec![
                control_item(control::Kind::SnapshotBegin, 0),
                control_item(control::Kind::SnapshotEnd, 0),
            ],
        );
        fake.scripts.lock().unwrap().insert(
            (EPOCH, 0),
            vec![
                control_item(control::Kind::SnapshotBegin, 0),
                item(
                    0,
                    "BTC-USD",
                    Body::ExecutionReport(report(
                        buy,
                        proto::ExecType::Restated,
                        OrderStatus::New,
                        (0, 0),
                        None,
                    )),
                ),
                item(
                    0,
                    "BTC-USD",
                    Body::ExecutionReport(report(
                        sell,
                        proto::ExecType::Restated,
                        OrderStatus::New,
                        (0, 0),
                        None,
                    )),
                ),
                control_item(control::Kind::SnapshotEnd, 0),
                item(1, "BTC-USD", Body::TradeSettlement(settlement)),
                item(
                    2,
                    "BTC-USD",
                    Body::ExecutionReport(report(
                        buy,
                        proto::ExecType::Trade,
                        OrderStatus::Filled,
                        (5, 1_000_000),
                        Some(trade_id),
                    )),
                ),
                item(
                    3,
                    "BTC-USD",
                    Body::ExecutionReport(report(
                        sell,
                        proto::ExecType::Trade,
                        OrderStatus::Filled,
                        (5, 1_000_000),
                        Some(trade_id),
                    )),
                ),
            ],
        );

        // the fake engine, served for real over gRPC
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let server = tokio::spawn({
            let engine = fake.clone();
            async move {
                tonic::transport::Server::builder()
                    .add_service(TradingEngineServer::new(engine))
                    .serve(([127, 0, 0, 1], port).into())
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let state = AppState {
            pg_connections: db.pool.clone(),
            tb_client: tb_client.clone(),
            engine_client: crate::service::EngineClient::new(&format!("http://127.0.0.1:{port}"))
                .unwrap(),
            books: Default::default(),
        };
        let (stop_tx, stop_rx) = watch::channel(false);
        let consumer = tokio::spawn(run(state.clone(), stop_rx));

        // wait until the three events are processed and the orders are accepted again
        let mut done = false;
        for _ in 0..100 {
            let cursor = load_cursor_from(p).await;
            if cursor.sequence == 3 && cursor.accepting {
                done = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        stop_tx.send(true).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(10), consumer)
            .await
            .expect("the consumer stops")
            .unwrap();
        assert!(result.is_ok(), "{result:?}");
        assert!(done, "the consumer didn't get to sequence 3");
        drop(state);

        // the engine was told about the market, and asked for what the stored epoch said
        assert_eq!(*fake.markets.lock().unwrap(), vec!["BTC-USD".to_string()]);
        assert_eq!(
            *fake.subscriptions.lock().unwrap(),
            vec![(0, 0), (EPOCH, 0)]
        );
        let cursor = load_cursor_from(p).await;
        assert_eq!(
            (cursor.epoch as u64, cursor.sequence, cursor.accepting),
            (EPOCH, 3, true)
        );

        // the orders: the command that was in flight is rejected and refunded, the one the engine
        // doesn't have is cancelled, the two that traded are filled
        let row = |id: Uuid| {
            let pool = db.pool.clone();
            async move {
                sqlx::query_as::<_, (String, Option<i16>, Option<i16>, String)>(
                    "SELECT submission::text, status, cancel_reason, filled_quote_quantity::text FROM orders WHERE order_id = $1",
                )
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
            }
        };
        assert_eq!(row(in_flight).await.0, "rejected");
        assert_eq!(
            row(gone).await,
            ("accepted".into(), Some(4), Some(7), "0".into())
        );
        assert_eq!(
            row(buy).await,
            ("accepted".into(), Some(3), None, "1000000".into())
        );
        assert_eq!(
            row(sell).await,
            ("accepted".into(), Some(3), None, "1000000".into())
        );

        // the trade was recorded
        let (trades, price, fee): (i64, String, String) =
            sqlx::query_as("SELECT count(*), max(price)::text, max(buyer_fee)::text FROM trades")
                .fetch_one(p)
                .await
                .unwrap();
        assert_eq!(
            (trades, price.as_str(), fee.as_str()),
            (1, "200000", "2000")
        );

        // and the money moved: base to the buyer, quote to the seller minus its fee, the fees to the fee
        // account, and what was left of every reservation went back to its owner
        assert_eq!(tb_balance(&tb_client, s.buyer.base).await, (5, 0));
        assert_eq!(
            tb_balance(&tb_client, s.seller.quote).await,
            (1_000_000, 1_000)
        );
        assert_eq!(tb_balance(&tb_client, s.fee_account).await, (3_000, 0));
        assert_eq!(
            tb_balance(&tb_client, s.buyer.quote).await,
            (5_000_000 + 1_000 + 500, 1_003_000 + 500)
        );
        assert_eq!(tb_balance(&tb_client, buy).await, (1_003_000, 1_003_000));
        assert_eq!(tb_balance(&tb_client, in_flight).await, (500, 500));

        server.abort();
        db.cleanup().await;
    }
}
