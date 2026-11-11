// Postgres side of the consumer. Quantities are u128 and the columns are NUMERIC(39, 0), which
// rust_decimal cannot hold, so every quantity crosses the wire as text and is cast in SQL.
use crate::consumer::plan::{MarketRow, OrderRow, OrderUpdate, TradeInsert};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::time::UNIX_EPOCH;
use uuid::Uuid;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CommitStats {
    pub trades_inserted: u64,
    pub orders_updated: u64,
    pub orders_skipped: u64,
}

fn parse_u128(s: &str) -> sqlx::Result<u128> {
    s.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

/// Which of these trades are already recorded.
pub async fn known_trade_ids(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<HashSet<Uuid>> {
    if ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<Uuid> =
        sqlx::query_scalar("SELECT trade_id FROM trades WHERE trade_id = ANY($1)")
            .bind(ids)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

type OrderTuple = (Uuid, Uuid, i32, Uuid, Uuid, i16, Option<String>, i32, i32);

/// The backend writes these columns while we run, so they are read fresh for every batch.
pub async fn load_orders(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<HashMap<Uuid, OrderRow>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<OrderTuple> = sqlx::query_as(
        "SELECT order_id, user_id, market_id, base_account_id, quote_account_id, side, \
         limit_price::text, maker_fee_bps, taker_fee_bps \
         FROM orders WHERE order_id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(
            |(order_id, user_id, market_id, base, quote, side, limit, maker, taker)| {
                let limit_price = limit.as_deref().map(parse_u128).transpose()?;
                Ok((
                    order_id,
                    OrderRow {
                        order_id,
                        user_id,
                        market_id,
                        base_account_id: base,
                        quote_account_id: quote,
                        buyer: side == 1,
                        limit_price,
                        maker_fee_bps: maker,
                        taker_fee_bps: taker,
                    },
                ))
            },
        )
        .collect()
}

pub async fn load_markets(
    pool: &PgPool,
    symbols: &[String],
) -> sqlx::Result<HashMap<String, MarketRow>> {
    if symbols.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(String, i32, i32, i32, Uuid)> = sqlx::query_as(
        "SELECT symbol, market_id, base_ledger_id, quote_ledger_id, fee_account_id \
         FROM markets WHERE symbol = ANY($1)",
    )
    .bind(symbols)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(symbol, market_id, base_ledger_id, quote_ledger_id, fee_account_id)| {
                (
                    symbol,
                    MarketRow {
                        market_id,
                        base_ledger_id,
                        quote_ledger_id,
                        fee_account_id,
                    },
                )
            },
        )
        .collect())
}

/// Writes a whole batch in one transaction.
///
/// `updates` must hold at most one entry per order
/// ([`build_plan`](crate::consumer::plan::build_plan) guarantees it). An update only applies if
/// the order isn't finished and its filled amounts don't go down; anything else is a replay of
/// something older and is skipped.
///
/// `status IS NULL` is explicit: a new order has no status, and `NULL NOT IN (...)` is never
/// true.
pub async fn commit(
    pool: &PgPool,
    trades: &[TradeInsert],
    updates: &[OrderUpdate],
) -> sqlx::Result<CommitStats> {
    let mut stats = CommitStats::default();
    let mut tx = pool.begin().await?;

    if !trades.is_empty() {
        let micros = |t: &TradeInsert| {
            t.engine_time
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_micros() as i64)
                .unwrap_or_default()
        };
        let result = sqlx::query(
            "INSERT INTO trades (trade_id, market_id, buyer_order_id, seller_order_id, price, \
             base_quantity, quote_quantity, buyer_fee, seller_fee, taker_side, engine_time) \
             SELECT r.trade_id, r.market_id, r.buyer, r.seller, r.price::numeric, r.base::numeric, \
             r.quote::numeric, r.bfee::numeric, r.sfee::numeric, r.taker, \
             TIMESTAMPTZ 'epoch' + r.us * INTERVAL '1 microsecond' \
             FROM unnest($1::uuid[], $2::int[], $3::uuid[], $4::uuid[], $5::text[], $6::text[], \
             $7::text[], $8::text[], $9::text[], $10::smallint[], $11::bigint[]) \
             AS r(trade_id, market_id, buyer, seller, price, base, quote, bfee, sfee, taker, us) \
             ON CONFLICT (trade_id) DO NOTHING",
        )
        .bind(trades.iter().map(|t| t.trade_id).collect::<Vec<_>>())
        .bind(trades.iter().map(|t| t.market_id).collect::<Vec<_>>())
        .bind(trades.iter().map(|t| t.buyer_order_id).collect::<Vec<_>>())
        .bind(trades.iter().map(|t| t.seller_order_id).collect::<Vec<_>>())
        .bind(
            trades
                .iter()
                .map(|t| t.price.to_string())
                .collect::<Vec<_>>(),
        )
        .bind(
            trades
                .iter()
                .map(|t| t.base_quantity.to_string())
                .collect::<Vec<_>>(),
        )
        .bind(
            trades
                .iter()
                .map(|t| t.quote_quantity.to_string())
                .collect::<Vec<_>>(),
        )
        .bind(
            trades
                .iter()
                .map(|t| t.buyer_fee.to_string())
                .collect::<Vec<_>>(),
        )
        .bind(
            trades
                .iter()
                .map(|t| t.seller_fee.to_string())
                .collect::<Vec<_>>(),
        )
        .bind(trades.iter().map(|t| t.taker_side).collect::<Vec<_>>())
        .bind(trades.iter().map(micros).collect::<Vec<_>>())
        .execute(&mut *tx)
        .await?;
        stats.trades_inserted = result.rows_affected();
    }

    if !updates.is_empty() {
        let result = sqlx::query(
            "UPDATE orders o SET status = r.status, cancel_reason = r.cancel_reason, \
             filled_base_quantity = r.fb::numeric, filled_quote_quantity = r.fq::numeric, \
             limit_price = COALESCE(r.lp::numeric, o.limit_price) \
             FROM unnest($1::uuid[], $2::smallint[], $3::smallint[], $4::text[], $5::text[], $6::text[]) \
             AS r(order_id, status, cancel_reason, fb, fq, lp) \
             WHERE o.order_id = r.order_id \
             AND (o.status IS NULL OR o.status NOT IN (3, 4, 5)) \
             AND o.filled_base_quantity <= r.fb::numeric \
             AND o.filled_quote_quantity <= r.fq::numeric",
        )
        .bind(updates.iter().map(|u| u.order_id).collect::<Vec<_>>())
        .bind(updates.iter().map(|u| u.status).collect::<Vec<_>>())
        .bind(updates.iter().map(|u| u.cancel_reason).collect::<Vec<_>>())
        .bind(updates.iter().map(|u| u.filled_base.to_string()).collect::<Vec<_>>())
        .bind(updates.iter().map(|u| u.filled_quote.to_string()).collect::<Vec<_>>())
        .bind(
            updates
                .iter()
                .map(|u| u.limit_price.map(|p| p.to_string()))
                .collect::<Vec<_>>(),
        )
        .execute(&mut *tx)
        .await?;
        stats.orders_updated = result.rows_affected();
        stats.orders_skipped = updates.len() as u64 - stats.orders_updated;
    }

    tx.commit().await?;
    Ok(stats)
}
