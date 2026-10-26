use std::collections::{HashMap, HashSet};

use cn_tigerbeetle as tb;
use num_traits::FromPrimitive;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::time::{Duration, sleep, timeout};
use uuid::Uuid;

use crate::domain::{Side, TransferCodeType};
use crate::v1::AppState;

pub type StreamError = Box<dyn std::error::Error + Send + Sync>;

// how many transfers go in one tigerbeetle request (the hard limit is 8189)
const MAX_TRANSFERS_PER_REQUEST: usize = 2000;

// the same input always gives the same transfer id, so processing an event twice makes tigerbeetle
// answer `exists` instead of moving the money twice. Never change this after deploying.
pub fn derived_id(tag: &str, id: u128, n: u8) -> u128 {
    let mut hasher = Sha256::new();
    hasher.update(tag.as_bytes());
    hasher.update(id.to_be_bytes());
    hasher.update([n]);
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_slice()[..16]);
    // tigerbeetle doesn't accept 0 or u128::MAX as ids
    u128::from_be_bytes(bytes).clamp(1, u128::MAX - 1)
}

// the biggest fee a rate allows: floor(quote * bps / 10000), without overflowing u128
pub fn fee_limit(quote: u128, bps: u32) -> u128 {
    let bps = bps as u128;
    (quote / 10_000)
        .saturating_mul(bps)
        .saturating_add((quote % 10_000) * bps / 10_000)
}

// what we need to know about an order to settle it. Numbers come as text because a NUMERIC(39,0)
// doesn't fit a Decimal when it's very big
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderInfo {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub market_id: i32,
    pub base_account_id: Uuid,
    pub quote_account_id: Uuid,
    pub side: Side,
    pub limit_price: Option<String>,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MarketInfo {
    pub market_id: i32,
    pub symbol: String,
    pub base_ledger_id: i32,
    pub quote_ledger_id: i32,
    pub fee_account_id: Uuid,
}

pub async fn load_orders(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, OrderInfo>, sqlx::Error> {
    let rows = sqlx::query_as::<_, OrderInfo>(
        "SELECT order_id, user_id, market_id, base_account_id, quote_account_id, side, \
                limit_price::text AS limit_price, maker_fee_bps, taker_fee_bps \
         FROM orders WHERE order_id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|o| (o.order_id, o)).collect())
}

pub async fn load_markets(pool: &PgPool) -> Result<Vec<MarketInfo>, sqlx::Error> {
    sqlx::query_as::<_, MarketInfo>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, fee_account_id FROM markets",
    )
    .fetch_all(pool)
    .await
}

// a TradeSettlement from the engine with every field checked and converted
#[derive(Debug, Clone)]
pub struct ParsedSettlement {
    pub trade_id: u128,
    pub price: u128,
    pub base_quantity: u128,
    pub quote_quantity: u128,
    pub taker_side: Side,
    pub buyer_order: u128,
    pub buyer_holder: u128,
    pub buyer_base_account: u128,
    pub buyer_quote_account: u128,
    pub buyer_fee: u128,
    pub seller_order: u128,
    pub seller_holder: u128,
    pub seller_base_account: u128,
    pub seller_quote_account: u128,
    pub seller_fee: u128,
    pub fee_account: u128,
}

// an absent field isn't a zero, it's an error
fn id_of(v: Option<proto::Id>, name: &str) -> Result<u128, String> {
    v.map(u128::from)
        .ok_or_else(|| format!("settlement without {name}"))
}

fn amount_of(v: Option<proto::Uint128>, name: &str) -> Result<u128, String> {
    v.map(u128::from)
        .ok_or_else(|| format!("settlement without {name}"))
}

pub fn parse_settlement(s: &proto::TradeSettlement) -> Result<ParsedSettlement, String> {
    Ok(ParsedSettlement {
        trade_id: id_of(s.trade_id, "trade_id")?,
        price: amount_of(s.price, "price")?,
        base_quantity: amount_of(s.base_quantity, "base_quantity")?,
        quote_quantity: amount_of(s.quote_quantity, "quote_quantity")?,
        taker_side: Side::from_i32(s.taker_side)
            .ok_or_else(|| format!("settlement with taker_side {}", s.taker_side))?,
        buyer_order: id_of(s.buyer_order_id, "buyer_order_id")?,
        buyer_holder: id_of(s.buyer_account_holder, "buyer_account_holder")?,
        buyer_base_account: id_of(s.buyer_base_account, "buyer_base_account")?,
        buyer_quote_account: id_of(s.buyer_quote_account, "buyer_quote_account")?,
        buyer_fee: amount_of(s.buyer_fee, "buyer_fee")?,
        seller_order: id_of(s.seller_order_id, "seller_order_id")?,
        seller_holder: id_of(s.seller_account_holder, "seller_account_holder")?,
        seller_base_account: id_of(s.seller_base_account, "seller_base_account")?,
        seller_quote_account: id_of(s.seller_quote_account, "seller_quote_account")?,
        seller_fee: amount_of(s.seller_fee, "seller_fee")?,
        fee_account: id_of(s.fee_account, "fee_account")?,
    })
}

// the message has to agree with the orders table before any money moves
pub fn validate_settlement(
    p: &ParsedSettlement,
    buyer: &OrderInfo,
    seller: &OrderInfo,
    market: &MarketInfo,
) -> Result<(), String> {
    if buyer.side != Side::Buy || seller.side != Side::Sell {
        return Err("the buyer isn't a buy order or the seller isn't a sell order".into());
    }
    if buyer.market_id != market.market_id || seller.market_id != market.market_id {
        return Err("the orders don't belong to the market of the event".into());
    }
    if buyer.user_id.as_u128() != p.buyer_holder
        || buyer.base_account_id.as_u128() != p.buyer_base_account
        || buyer.quote_account_id.as_u128() != p.buyer_quote_account
    {
        return Err("the buyer's user or accounts don't match its order".into());
    }
    if seller.user_id.as_u128() != p.seller_holder
        || seller.base_account_id.as_u128() != p.seller_base_account
        || seller.quote_account_id.as_u128() != p.seller_quote_account
    {
        return Err("the seller's user or accounts don't match its order".into());
    }
    if market.fee_account_id.as_u128() != p.fee_account {
        return Err("the fee account isn't the market's".into());
    }

    // a buyer never pays more than its limit, a seller never gets less
    if let Some(limit) = &buyer.limit_price {
        let limit: u128 = limit.parse().map_err(|_| "unreadable buyer limit price")?;
        if p.price > limit {
            return Err(format!(
                "price {} is above the buyer's limit {limit}",
                p.price
            ));
        }
    }
    if let Some(limit) = &seller.limit_price {
        let limit: u128 = limit.parse().map_err(|_| "unreadable seller limit price")?;
        if p.price < limit {
            return Err(format!(
                "price {} is below the seller's limit {limit}",
                p.price
            ));
        }
    }

    // each side is charged the rate of its role in this fill
    let (buyer_bps, seller_bps) = match p.taker_side {
        Side::Buy => (buyer.taker_fee_bps, seller.maker_fee_bps),
        Side::Sell => (buyer.maker_fee_bps, seller.taker_fee_bps),
    };
    let buyer_max = fee_limit(p.quote_quantity, u32::try_from(buyer_bps).unwrap_or(0));
    let seller_max = fee_limit(p.quote_quantity, u32::try_from(seller_bps).unwrap_or(0));
    if p.buyer_fee > buyer_max {
        return Err(format!("buyer fee {} is above {buyer_max}", p.buyer_fee));
    }
    if p.seller_fee > seller_max {
        return Err(format!("seller fee {} is above {seller_max}", p.seller_fee));
    }
    Ok(())
}

// the legs of one trade, in the order the guide asks for (leg 2 funds the seller's fee in leg 4).
// The hold account of an order has the id of the order.
pub fn trade_transfers(
    p: &ParsedSettlement,
    market: &MarketInfo,
    generation: u8,
) -> Vec<tb::Transfer> {
    let base_ledger = market.base_ledger_id as u32;
    let quote_ledger = market.quote_ledger_id as u32;
    let leg =
        |n: u8, debit: u128, credit: u128, amount: u128, ledger: u32, code: TransferCodeType| {
            tb::Transfer {
                id: derived_id("trade-leg", p.trade_id, generation * 8 + n),
                debit_account_id: debit,
                credit_account_id: credit,
                amount,
                user_data_128: p.trade_id,
                ledger,
                code: code as u16,
                ..Default::default()
            }
        };

    let mut legs = vec![
        leg(
            1,
            p.seller_order,
            p.buyer_base_account,
            p.base_quantity,
            base_ledger,
            TransferCodeType::Trade,
        ),
        leg(
            2,
            p.buyer_order,
            p.seller_quote_account,
            p.quote_quantity,
            quote_ledger,
            TransferCodeType::Trade,
        ),
    ];
    if p.buyer_fee > 0 {
        legs.push(leg(
            3,
            p.buyer_order,
            p.fee_account,
            p.buyer_fee,
            quote_ledger,
            TransferCodeType::FeeCollection,
        ));
    }
    if p.seller_fee > 0 {
        legs.push(leg(
            4,
            p.seller_quote_account,
            p.fee_account,
            p.seller_fee,
            quote_ledger,
            TransferCodeType::FeeCollection,
        ));
    }
    legs
}

// moves whatever is left in the order's hold account back to the user. It's a balancing debit with
// a huge amount: tigerbeetle moves exactly what's left, and the id comes from the order so a repeated
// refund is `exists`.
pub fn refund_transfer(o: &OrderInfo, m: &MarketInfo, generation: u8) -> tb::Transfer {
    let (account, ledger) = match o.side {
        Side::Buy => (o.quote_account_id, m.quote_ledger_id),
        Side::Sell => (o.base_account_id, m.base_ledger_id),
    };
    tb::Transfer {
        id: derived_id("refund", o.order_id.as_u128(), generation * 8),
        debit_account_id: o.order_id.as_u128(),
        credit_account_id: account.as_u128(),
        amount: u128::MAX,
        user_data_128: o.order_id.as_u128(),
        ledger: ledger as u32,
        code: TransferCodeType::Refund as u16,
        flags: tb::TransferFlags::BalancingDebit,
        ..Default::default()
    }
}

// The money an event moves. It's kept (and not just its transfers) because tigerbeetle never lets a
// transfer id be used again after it failed once (`IdAlreadyFailed`): if the cause is fixed later
// (a missing account created, a bad amount corrected), the same event is tried again with the ids
// of the next generation. An id that succeeded is never in a different generation than its event's
// applied one, so a replay always finds it as `exists` first.
#[derive(Debug, Clone)]
pub enum Money {
    Trade(ParsedSettlement, MarketInfo),
    Refund(OrderInfo, MarketInfo),
}

// ids are derived from generation * 8 + leg, so it has to stay below 32
const MAX_GENERATION: u8 = 30;

impl Money {
    pub fn transfers(&self, generation: u8) -> Vec<tb::Transfer> {
        match self {
            Money::Trade(p, market) => trade_transfers(p, market, generation),
            Money::Refund(order, market) => vec![refund_transfer(order, market, generation)],
        }
    }

    // the id of the first transfer of a generation: a chain is atomic, so if it exists the whole
    // generation was applied
    fn marker(&self, generation: u8) -> u128 {
        self.transfers(generation)[0].id
    }
}

// the events that were applied already with the ids of a later generation than the first. They have
// to be known before anything is sent: the first generation's ids of such an event may not have
// failed (only the transfer that failed is remembered), so sending it again could settle it twice.
async fn applied_in_a_later_generation(
    tb_client: &tb::Client,
    events: &[Option<Money>],
) -> Result<HashSet<usize>, StreamError> {
    let mut asked: Vec<(u128, usize)> = Vec::new();
    for (i, event) in events.iter().enumerate() {
        if let Some(money) = event {
            asked.extend((1..=MAX_GENERATION).map(|g| (money.marker(g), i)));
        }
    }

    let mut applied = HashSet::new();
    for chunk in asked.chunks(4000) {
        let ids: Vec<u128> = chunk.iter().map(|(id, _)| *id).collect();
        let found: HashSet<u128> =
            timeout(Duration::from_secs(5), tb_client.lookup_transfers(&ids))
                .await
                .map_err(|_| "tigerbeetle timed out looking up transfers")?
                .map_err(|e| format!("tigerbeetle lookup failed: {e:?}"))?
                .into_iter()
                .map(|t| t.id)
                .collect();
        applied.extend(
            chunk
                .iter()
                .filter(|(id, _)| found.contains(id))
                .map(|(_, i)| *i),
        );
    }
    Ok(applied)
}

#[derive(Debug)]
pub struct GroupFailure {
    pub group: usize,
    pub result: tb::CreateTransferResult,
    pub index: usize, // of the transfer inside the group
}

impl GroupFailure {
    pub fn detail(&self) -> String {
        format!("{:?} at transfer {}", self.result, self.index)
    }
}

enum ChainResult {
    Applied,
    Failed(tb::CreateTransferResult, usize), // the first failure tigerbeetle reported
    // tigerbeetle only said `exists` / `linked_event_failed`, and the transfers aren't all there
    Unverified,
}

// Settles the money of a list of events (the ones that move none are None). First all of it goes in
// ONE linked chain, so a batch is applied or nothing is. If that doesn't work, the events go one by
// one, in order, and it stops at the first one that fails: the ones before it are applied.
// An event that was applied already (a replay after a crash) counts as applied.
//
// Two things about tigerbeetle that shape this:
// - A failed chain fails ALL of its transfers, and an id that failed can never be used again
//   (`IdAlreadyFailed`). So when that's what an event gets, it goes again with the ids of the next
//   generation. This is also how an event is retried after whatever made it fail has been fixed.
// - Only the transfer that fails is remembered, not the rest of its chain.
// - In a chain, if the first transfer already exists, the rest comes back as `linked_event_failed`
//   without being looked at. That says nothing about them, so it has to be checked.
pub async fn submit_money(
    tb_client: &tb::Client,
    events: &[Option<Money>],
    tolerate_missing_hold: bool,
) -> Result<Option<GroupFailure>, StreamError> {
    let applied_later = applied_in_a_later_generation(tb_client, events).await?;
    let live: Vec<usize> = (0..events.len())
        .filter(|&i| events[i].is_some() && !applied_later.contains(&i))
        .collect();

    if live.len() > 1 {
        let everything: Vec<tb::Transfer> = live
            .iter()
            .flat_map(|&i| events[i].iter().flat_map(|m| m.transfers(0)))
            .collect();
        if let ChainResult::Applied =
            send_chain(tb_client, everything, tolerate_missing_hold).await?
        {
            return Ok(None);
        }
    }

    for &i in &live {
        let Some(money) = &events[i] else { continue };
        let mut generation = 0u8;
        loop {
            let chain = money.transfers(generation);
            match send_chain(tb_client, chain, tolerate_missing_hold).await? {
                ChainResult::Applied => break,
                ChainResult::Failed(tb::CreateTransferResult::IdAlreadyFailed, _)
                    if generation < MAX_GENERATION =>
                {
                    generation += 1;
                }
                ChainResult::Failed(result, index) => {
                    return Ok(Some(GroupFailure {
                        group: i,
                        result,
                        index,
                    }));
                }
                ChainResult::Unverified => {
                    return Ok(Some(GroupFailure {
                        group: i,
                        result: tb::CreateTransferResult::LinkedEventFailed,
                        index: 0,
                    }));
                }
            }
        }
    }
    Ok(None)
}

// one linked chain. Err only for connection problems (after retrying)
async fn send_chain(
    tb_client: &tb::Client,
    mut transfers: Vec<tb::Transfer>,
    tolerate_missing_hold: bool,
) -> Result<ChainResult, StreamError> {
    let ids: Vec<u128> = transfers.iter().map(|t| t.id).collect();
    let last = transfers.len() - 1;
    for t in &mut transfers[..last] {
        t.flags |= tb::TransferFlags::Linked;
    }

    let mut attempt = 0u32;
    let results = loop {
        attempt += 1;
        match timeout(
            Duration::from_secs(5),
            tb_client.create_transfers(&transfers),
        )
        .await
        {
            Ok(Ok(results)) => break results,
            Ok(Err(e)) if attempt >= 5 => {
                return Err(format!("tigerbeetle failed 5 times: {e:?}").into());
            }
            Err(_) if attempt >= 5 => return Err("tigerbeetle timed out 5 times".into()),
            Ok(Err(e)) => eprintln!("SETTLEMENT: tigerbeetle failed (attempt {attempt}): {e:?}"),
            Err(_) => eprintln!("SETTLEMENT: tigerbeetle timed out (attempt {attempt})"),
        }
        sleep(Duration::from_millis(500) * attempt).await;
    };

    // the results only list the events that didn't succeed
    let missing_hold = |r: &tb::CreateTransfersResult| {
        tolerate_missing_hold && r.result == tb::CreateTransferResult::DebitAccountNotFound
    };
    let harmless = |r: &tb::CreateTransfersResult| {
        matches!(
            r.result,
            tb::CreateTransferResult::Ok
                | tb::CreateTransferResult::Exists
                | tb::CreateTransferResult::LinkedEventFailed
        ) || missing_hold(r)
    };
    if let Some(r) = results.iter().find(|r| !harmless(r)) {
        return Ok(ChainResult::Failed(r.result, r.index));
    }
    if results.is_empty() {
        return Ok(ChainResult::Applied);
    }
    // nothing to refund, there's no account (the only transfer of the chain)
    if transfers.len() == 1 && results.iter().any(missing_hold) {
        return Ok(ChainResult::Applied);
    }

    // only `exists` / `linked_event_failed`: it was applied before, or the first transfer exists and
    // hides what happens to the others. Looking them up is the only way to know.
    let found = timeout(Duration::from_secs(5), tb_client.lookup_transfers(&ids))
        .await
        .map_err(|_| "tigerbeetle timed out looking up transfers")?
        .map_err(|e| format!("tigerbeetle lookup failed: {e:?}"))?;
    Ok(if found.len() == ids.len() {
        ChainResult::Applied
    } else {
        ChainResult::Unverified
    })
}

// refunds what is left of the reservation of each order. With `tolerate_missing_hold` an order whose
// hold account was never created is skipped (there's nothing to give back). The hold accounts are
// looked up first so that no transfer is sent for one that doesn't exist: tigerbeetle would remember
// that attempt as failed and refuse the same refund once the account does exist.
pub async fn refund_orders(
    state: &AppState,
    ids: &[Uuid],
    tolerate_missing_hold: bool,
) -> Result<(), StreamError> {
    if ids.is_empty() {
        return Ok(());
    }
    let orders = load_orders(&state.pg_connections, ids).await?;
    let markets: HashMap<i32, MarketInfo> = load_markets(&state.pg_connections)
        .await?
        .into_iter()
        .map(|m| (m.market_id, m))
        .collect();

    let mut wanted: Vec<Money> = Vec::new();
    for id in ids {
        let Some(order) = orders.get(id) else {
            eprintln!("SETTLEMENT: can't refund order {id}, it isn't in the database");
            continue;
        };
        let market = markets
            .get(&order.market_id)
            .ok_or_else(|| format!("order {id} belongs to an unknown market"))?;
        wanted.push(Money::Refund(order.clone(), market.clone()));
    }

    for chunk in wanted.chunks(MAX_TRANSFERS_PER_REQUEST) {
        let holds: Vec<u128> = chunk
            .iter()
            .filter_map(|m| match m {
                Money::Refund(o, _) => Some(o.order_id.as_u128()),
                _ => None,
            })
            .collect();
        let found: HashSet<u128> = state
            .tb_client
            .lookup_accounts(&holds)
            .await
            .map_err(|e| format!("tigerbeetle lookup failed: {e:?}"))?
            .into_iter()
            .map(|a| a.id)
            .collect();

        let mut events: Vec<Option<Money>> = Vec::new();
        for money in chunk {
            let Money::Refund(order, _) = money else {
                continue;
            };
            if found.contains(&order.order_id.as_u128()) {
                events.push(Some(money.clone()));
            } else if !tolerate_missing_hold {
                return Err(
                    format!("order {} has no hold account to refund", order.order_id).into(),
                );
            }
        }

        if let Some(f) = submit_money(&state.tb_client, &events, false).await? {
            let order = match &events[f.group] {
                Some(Money::Refund(o, _)) => o.order_id,
                _ => Uuid::nil(),
            };
            return Err(format!("the refund of order {order} failed: {}", f.detail()).into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(side: Side, limit: Option<&str>) -> OrderInfo {
        OrderInfo {
            order_id: Uuid::from_u128(10 + side as u128),
            user_id: Uuid::from_u128(100 + side as u128),
            market_id: 1,
            base_account_id: Uuid::from_u128(200 + side as u128),
            quote_account_id: Uuid::from_u128(300 + side as u128),
            side,
            limit_price: limit.map(String::from),
            maker_fee_bps: 10,
            taker_fee_bps: 20,
        }
    }

    fn market() -> MarketInfo {
        MarketInfo {
            market_id: 1,
            symbol: "BTC-USD".into(),
            base_ledger_id: 7,
            quote_ledger_id: 8,
            fee_account_id: Uuid::from_u128(999),
        }
    }

    // a buy order 11 and a sell order 12 that traded 5 base for 1_000_000 quote
    fn settlement(buyer: &OrderInfo, seller: &OrderInfo) -> ParsedSettlement {
        ParsedSettlement {
            trade_id: 77,
            price: 200_000,
            base_quantity: 5,
            quote_quantity: 1_000_000,
            taker_side: Side::Buy,
            buyer_order: buyer.order_id.as_u128(),
            buyer_holder: buyer.user_id.as_u128(),
            buyer_base_account: buyer.base_account_id.as_u128(),
            buyer_quote_account: buyer.quote_account_id.as_u128(),
            buyer_fee: 2_000, // taker: 20 bps of 1_000_000
            seller_order: seller.order_id.as_u128(),
            seller_holder: seller.user_id.as_u128(),
            seller_base_account: seller.base_account_id.as_u128(),
            seller_quote_account: seller.quote_account_id.as_u128(),
            seller_fee: 1_000, // maker: 10 bps
            fee_account: 999,
        }
    }

    #[test]
    fn derived_ids_are_stable_and_distinct() {
        assert_eq!(derived_id("trade-leg", 5, 1), derived_id("trade-leg", 5, 1));
        assert_ne!(derived_id("trade-leg", 5, 1), derived_id("trade-leg", 5, 2));
        assert_ne!(derived_id("trade-leg", 5, 1), derived_id("trade-leg", 6, 1));
        assert_ne!(derived_id("trade-leg", 5, 1), derived_id("refund", 5, 1));
        let id = derived_id("refund", 0, 0);
        assert!(id != 0 && id != u128::MAX);
    }

    #[test]
    fn fee_limit_rounds_down_and_does_not_overflow() {
        assert_eq!(fee_limit(1_000_000, 20), 2_000);
        assert_eq!(fee_limit(9_999, 1), 0);
        assert_eq!(fee_limit(10_001, 1), 1);
        assert_eq!(fee_limit(u128::MAX, u32::MAX), u128::MAX);
    }

    #[test]
    fn a_consistent_settlement_is_valid() {
        let (b, s) = (
            order(Side::Buy, Some("200000")),
            order(Side::Sell, Some("150000")),
        );
        assert_eq!(
            validate_settlement(&settlement(&b, &s), &b, &s, &market()),
            Ok(())
        );
    }

    #[test]
    fn settlement_checks_catch_mismatches() {
        let (b, s) = (order(Side::Buy, Some("199999")), order(Side::Sell, None));
        assert!(validate_settlement(&settlement(&b, &s), &b, &s, &market()).is_err()); // above buyer limit

        let (b, s) = (order(Side::Buy, None), order(Side::Sell, Some("200001")));
        assert!(validate_settlement(&settlement(&b, &s), &b, &s, &market()).is_err()); // below seller limit

        let (b, s) = (order(Side::Buy, None), order(Side::Sell, None));
        let mut p = settlement(&b, &s);
        p.buyer_fee = 2_001; // more than the taker rate allows
        assert!(validate_settlement(&p, &b, &s, &market()).is_err());

        let mut p = settlement(&b, &s);
        p.taker_side = Side::Sell; // now the buyer is the maker: 10 bps = 1_000, 2_000 is too much
        assert!(validate_settlement(&p, &b, &s, &market()).is_err());

        let mut p = settlement(&b, &s);
        p.fee_account = 1;
        assert!(validate_settlement(&p, &b, &s, &market()).is_err());

        let mut p = settlement(&b, &s);
        p.buyer_quote_account += 1;
        assert!(validate_settlement(&p, &b, &s, &market()).is_err());

        assert!(validate_settlement(&settlement(&s, &b), &s, &b, &market()).is_err()); // sides swapped
    }

    #[test]
    fn a_trade_has_the_legs_of_the_guide() {
        let (b, s) = (order(Side::Buy, None), order(Side::Sell, None));
        let p = settlement(&b, &s);
        let legs = trade_transfers(&p, &market(), 0);
        assert_eq!(legs.len(), 4);
        // 1: base from the seller's hold to the buyer
        assert_eq!(
            (legs[0].debit_account_id, legs[0].credit_account_id),
            (p.seller_order, p.buyer_base_account)
        );
        assert_eq!((legs[0].amount, legs[0].ledger), (5, 7));
        // 2: quote from the buyer's hold to the seller
        assert_eq!(
            (legs[1].debit_account_id, legs[1].credit_account_id),
            (p.buyer_order, p.seller_quote_account)
        );
        assert_eq!((legs[1].amount, legs[1].ledger), (1_000_000, 8));
        // 3: buyer fee from its hold, 4: seller fee from what leg 2 gave it
        assert_eq!(
            (legs[2].debit_account_id, legs[2].credit_account_id),
            (p.buyer_order, 999)
        );
        assert_eq!(
            (legs[3].debit_account_id, legs[3].credit_account_id),
            (p.seller_quote_account, 999)
        );

        let mut ids: Vec<u128> = legs.iter().map(|t| t.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn zero_fee_legs_are_skipped() {
        let (b, s) = (order(Side::Buy, None), order(Side::Sell, None));
        let mut p = settlement(&b, &s);
        p.buyer_fee = 0;
        let legs = trade_transfers(&p, &market(), 0);
        assert_eq!(legs.len(), 3);
        // the legs keep their ids, so leg 4 is the same transfer with or without leg 3
        assert_eq!(legs[2].id, derived_id("trade-leg", 77, 4));
    }

    #[test]
    fn refunds_go_back_to_the_reserved_asset() {
        let m = market();
        let buy = refund_transfer(&order(Side::Buy, None), &m, 0);
        assert_eq!((buy.credit_account_id, buy.ledger), (301, 8)); // quote account
        let sell = refund_transfer(&order(Side::Sell, None), &m, 0);
        assert_eq!((sell.credit_account_id, sell.ledger), (202, 7)); // base account
        assert_eq!(buy.debit_account_id, 11); // the hold account has the id of the order
        assert!(buy.flags.contains(tb::TransferFlags::BalancingDebit));
    }
}

// The chains against a real tigerbeetle. Set TEST_TB_ADDRESS (for example 127.0.0.1:3000, a
// single-replica development server with cluster id 0); without it this test does nothing.
#[cfg(test)]
mod tigerbeetle_tests {
    use super::*;

    const BASE: u32 = 3;
    const QUOTE: u32 = 1;

    async fn account(client: &tb::Client, ledger: u32, hold: bool) -> u128 {
        let id = tb::id();
        let results = client
            .create_accounts(&[tb::Account {
                id,
                ledger,
                code: 1,
                // user and hold accounts can't go negative, only the source of the money can
                flags: if hold {
                    tb::AccountFlags::DebitsMustNotExceedCredits
                } else {
                    tb::AccountFlags::empty()
                },
                ..Default::default()
            }])
            .await
            .expect("create account");
        assert!(results.is_empty(), "{results:?}");
        id
    }

    async fn fund(client: &tb::Client, from: u128, to: u128, amount: u128, ledger: u32) {
        let results = client
            .create_transfers(&[tb::Transfer {
                id: tb::id(),
                debit_account_id: from,
                credit_account_id: to,
                amount,
                ledger,
                code: 1,
                ..Default::default()
            }])
            .await
            .expect("create transfer");
        assert!(results.is_empty(), "{results:?}");
    }

    // (credits posted, debits posted)
    async fn balance(client: &tb::Client, id: u128) -> (u128, u128) {
        let a = client.lookup_accounts(&[id]).await.expect("lookup")[0];
        (a.credits_posted, a.debits_posted)
    }

    fn order(id: u128, side: Side, base: u128, quote: u128) -> OrderInfo {
        OrderInfo {
            order_id: Uuid::from_u128(id),
            user_id: Uuid::nil(),
            market_id: 1,
            base_account_id: Uuid::from_u128(base),
            quote_account_id: Uuid::from_u128(quote),
            side,
            limit_price: None,
            maker_fee_bps: 10,
            taker_fee_bps: 20,
        }
    }

    #[tokio::test]
    async fn trades_and_refunds_settle_once() {
        let Ok(address) = std::env::var("TEST_TB_ADDRESS") else {
            return;
        };
        let client = tb::Client::new(0, &address).expect("connect to tigerbeetle");

        let source_base = account(&client, BASE, false).await;
        let source_quote = account(&client, QUOTE, false).await;
        let (buyer_base, buyer_quote) = (
            account(&client, BASE, true).await,
            account(&client, QUOTE, true).await,
        );
        let (seller_base, seller_quote) = (
            account(&client, BASE, true).await,
            account(&client, QUOTE, true).await,
        );
        let fee = account(&client, QUOTE, true).await;
        // the hold accounts have the id of their order
        let buyer_hold = account(&client, QUOTE, true).await;
        let seller_hold = account(&client, BASE, true).await;

        // the buyer reserves 1_000_000 plus a 3_000 allowance for the fee, the seller reserves 5 base
        fund(&client, source_quote, buyer_quote, 5_000_000, QUOTE).await;
        fund(&client, buyer_quote, buyer_hold, 1_003_000, QUOTE).await;
        fund(&client, source_base, seller_base, 50, BASE).await;
        fund(&client, seller_base, seller_hold, 5, BASE).await;

        let market = MarketInfo {
            market_id: 1,
            symbol: "BTC-USD".into(),
            base_ledger_id: BASE as i32,
            quote_ledger_id: QUOTE as i32,
            fee_account_id: Uuid::from_u128(fee),
        };
        let trade = |trade_id: u128, base_quantity: u128, quote_quantity: u128| ParsedSettlement {
            trade_id,
            price: 200_000,
            base_quantity,
            quote_quantity,
            taker_side: Side::Buy,
            buyer_order: buyer_hold,
            buyer_holder: 0,
            buyer_base_account: buyer_base,
            buyer_quote_account: buyer_quote,
            buyer_fee: 2_000,
            seller_order: seller_hold,
            seller_holder: 0,
            seller_base_account: seller_base,
            seller_quote_account: seller_quote,
            seller_fee: 1_000,
            fee_account: fee,
        };
        let settle = |p: ParsedSettlement| Some(Money::Trade(p, market.clone()));

        // the trade settles: base to the buyer, quote to the seller, and both fees to the fee account
        let first = settle(trade(tb::id(), 5, 1_000_000));
        assert!(
            submit_money(&client, std::slice::from_ref(&first), false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(balance(&client, buyer_base).await, (5, 0));
        assert_eq!(balance(&client, seller_quote).await, (1_000_000, 1_000)); // the fee came out of it
        assert_eq!(balance(&client, fee).await, (3_000, 0));

        // processing it again (a crash before the cursor moved) changes nothing and isn't an error
        assert!(
            submit_money(&client, std::slice::from_ref(&first), false)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            submit_money(&client, &[first.clone(), first.clone(), None], false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(balance(&client, buyer_base).await, (5, 0));
        assert_eq!(balance(&client, fee).await, (3_000, 0));

        // the finished order gets what is left of its reservation: 1_003_000 - 1_000_000 - 2_000 = 1_000
        let refund = Some(Money::Refund(
            order(buyer_hold, Side::Buy, buyer_base, buyer_quote),
            market.clone(),
        ));
        assert!(
            submit_money(&client, std::slice::from_ref(&refund), false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(balance(&client, buyer_quote).await, (5_001_000, 1_003_000)); // 1_000 came back
        assert_eq!(balance(&client, buyer_hold).await, (1_003_000, 1_003_000)); // and nothing is left
        assert!(
            submit_money(&client, &[refund], false)
                .await
                .unwrap()
                .is_none()
        ); // a second refund is `exists`

        // an order that used everything has nothing left to give back, and that isn't an error either
        let nothing_left = Some(Money::Refund(
            order(seller_hold, Side::Sell, seller_base, seller_quote),
            market.clone(),
        ));
        assert!(
            submit_money(&client, &[nothing_left], false)
                .await
                .unwrap()
                .is_none()
        );

        // a batch where the second item can't be settled (the buyer's hold account is empty now): the
        // first item is applied, and the failure points at the second
        let good = settle(ParsedSettlement {
            buyer_fee: 0,
            seller_fee: 0,
            ..trade(tb::id(), 0, 0)
        });
        let bad = settle(trade(tb::id(), 1, 100));
        let failure = submit_money(&client, &[good.clone(), bad.clone()], false)
            .await
            .unwrap()
            .expect("the second one fails");
        assert_eq!(failure.group, 1);
        assert!(
            submit_money(&client, &[good], false)
                .await
                .unwrap()
                .is_none()
        ); // the first was applied

        // tigerbeetle never lets a failed transfer id be used again, so once the cause is fixed the
        // same trade settles with the ids of the next generation: money for the hold account
        fund(&client, source_quote, buyer_quote, 1_000_000, QUOTE).await;
        fund(&client, buyer_quote, buyer_hold, 1_000, QUOTE).await;
        fund(&client, source_base, seller_base, 1, BASE).await;
        fund(&client, seller_base, seller_hold, 1, BASE).await;
        let bad_trade = trade(tb::id(), 1, 100);
        let broken = settle(ParsedSettlement {
            quote_quantity: 2_000,
            ..bad_trade.clone()
        }); // the hold has 1_000
        let failed = submit_money(&client, &[broken], false)
            .await
            .unwrap()
            .expect("not enough in the hold account");
        assert_eq!(failed.result, tb::CreateTransferResult::ExceedsCredits);
        // the engine fixes its message and the same trade id is tried again
        let fixed = settle(ParsedSettlement {
            quote_quantity: 100,
            buyer_fee: 0,
            seller_fee: 0,
            ..bad_trade
        });
        assert!(
            submit_money(&client, std::slice::from_ref(&fixed), false)
                .await
                .unwrap()
                .is_none()
        );
        let replay = submit_money(&client, &[fixed], false).await.unwrap(); // a replay now
        assert!(replay.is_none(), "{replay:?}");
        assert_eq!(balance(&client, buyer_base).await, (6, 0));

        // a batch whose first event was applied already and whose second is new: the `exists` of the
        // first must not hide the second
        fund(&client, source_quote, buyer_quote, 1_000_000, QUOTE).await;
        fund(&client, buyer_quote, buyer_hold, 300, QUOTE).await;
        fund(&client, source_base, seller_base, 2, BASE).await;
        fund(&client, seller_base, seller_hold, 2, BASE).await;
        let plain = |base, quote| ParsedSettlement {
            buyer_fee: 0,
            seller_fee: 0,
            ..trade(tb::id(), base, quote)
        };
        let (a, b) = (settle(plain(1, 100)), settle(plain(1, 100)));
        assert!(
            submit_money(&client, std::slice::from_ref(&a), false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(balance(&client, buyer_base).await, (7, 0));
        assert!(
            submit_money(&client, &[a, b], false)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(balance(&client, buyer_base).await, (8, 0));

        // an order without a hold account is an error, unless that's tolerated
        let ghost = Some(Money::Refund(
            order(tb::id(), Side::Buy, buyer_base, buyer_quote),
            market.clone(),
        ));
        let failure = submit_money(&client, std::slice::from_ref(&ghost), false)
            .await
            .unwrap()
            .expect("no hold account");
        assert_eq!(
            failure.result,
            tb::CreateTransferResult::DebitAccountNotFound
        );
        assert!(
            submit_money(&client, &[ghost], true)
                .await
                .unwrap()
                .is_none()
        );
    }
}

// A real postgres for the tests that check the SQL. Set TEST_DATABASE_URL to a server you can create
// databases in (for example postgres://postgres:pw@127.0.0.1:5432/postgres); without it those tests
// do nothing. Every test gets its own database with all the migrations.
#[cfg(test)]
pub(crate) mod test_db {
    use sqlx::{Connection, PgConnection, PgPool};
    use uuid::Uuid;

    pub struct TestDb {
        pub pool: PgPool,
        name: String,
        admin_url: String,
    }

    impl TestDb {
        pub async fn new() -> Option<TestDb> {
            let admin_url = std::env::var("TEST_DATABASE_URL").ok()?;
            let name = format!("test_{}", Uuid::new_v4().simple());
            let mut admin = PgConnection::connect(&admin_url)
                .await
                .expect("connect to the test server");
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .execute(&mut admin)
                .await
                .expect("create the test database");

            let (server, _) = admin_url.rsplit_once('/').expect("a database url");
            let pool = PgPool::connect(&format!("{server}/{name}"))
                .await
                .expect("connect to the test database");
            sqlx::migrate!()
                .run(&pool)
                .await
                .expect("migrate the test database");
            Some(TestDb {
                pool,
                name,
                admin_url,
            })
        }

        pub async fn cleanup(self) {
            self.pool.close().await;
            let mut admin = PgConnection::connect(&self.admin_url)
                .await
                .expect("connect to the test server");
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            )))
            .execute(&mut admin)
            .await
            .expect("drop the test database");
        }
    }

    #[derive(Clone)]
    pub struct Party {
        pub user: Uuid,
        pub base: Uuid,
        pub quote: Uuid,
    }

    pub struct Seed {
        pub market_id: i32,
        pub fee_account: Uuid,
        pub buyer: Party,
        pub seller: Party,
    }

    async fn account(pool: &PgPool, user: Uuid, ledger: i32, code: i16) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO accounts (account_id, account_name, account_ledger_id, account_code_type, \
             account_user_id, account_status) VALUES ($1, 'test', $2, $3, $4, 'active')",
        )
        .bind(id)
        .bind(ledger)
        .bind(code)
        .bind(user)
        .execute(pool)
        .await
        .expect("insert account");
        id
    }

    async fn party(pool: &PgPool, name: &str) -> Party {
        let user: Uuid =
            sqlx::query_scalar("INSERT INTO users (user_name) VALUES ($1) RETURNING user_id")
                .bind(name)
                .fetch_one(pool)
                .await
                .expect("insert user");
        // the migrations create USD = 1, EUR = 2, BTC = 3
        Party {
            user,
            base: account(pool, user, 3, 1).await,
            quote: account(pool, user, 1, 1).await,
        }
    }

    // BTC-USD with a buyer, a seller and the fee account
    pub async fn seed(pool: &PgPool) -> Seed {
        let system: Uuid = sqlx::query_scalar(
            "INSERT INTO users (user_name, is_system) VALUES ('system', TRUE) RETURNING user_id",
        )
        .fetch_one(pool)
        .await
        .expect("insert system user");
        let fee_account = account(pool, system, 1, 2).await;
        let market_id: i32 = sqlx::query_scalar(
            "INSERT INTO markets (symbol, base_ledger_id, quote_ledger_id, price_decimals, fee_account_id, \
             status, min_base_quantity, min_quote_quantity, maker_fee_bps, taker_fee_bps) \
             VALUES ('BTC-USD', 3, 1, 2, $1, 'trading', 1, 1, 10, 20) RETURNING market_id",
        )
        .bind(fee_account).fetch_one(pool).await.expect("insert market");
        Seed {
            market_id,
            fee_account,
            buyer: party(pool, "buyer").await,
            seller: party(pool, "seller").await,
        }
    }

    // an order of the party, created `age_seconds` ago. `side` is 1 (buy) or 2 (sell)
    pub async fn order(
        pool: &PgPool,
        seed: &Seed,
        side: i16,
        submission: &str,
        status: Option<i16>,
        limit_price: Option<&str>,
        age_seconds: i32,
    ) -> Uuid {
        let who = if side == 1 { &seed.buyer } else { &seed.seller };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO orders (order_id, idempotency_key, command_id, user_id, market_id, \
                 base_account_id, quote_account_id, side, order_type, time_in_force, base_quantity, \
                 limit_price, maker_fee_bps, taker_fee_bps, reserved_amount, submission, status, \
                 created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 1, 5, $9::text::numeric, 10, 20, 1000, \
                 $10::order_submission, $11, now() - $12::int * interval '1 second', \
                 now() - $12::int * interval '1 second')",
        )
        .bind(id).bind(Uuid::new_v4()).bind(Uuid::new_v4())
        .bind(who.user).bind(seed.market_id).bind(who.base).bind(who.quote)
        .bind(side).bind(limit_price).bind(submission).bind(status).bind(age_seconds)
        .execute(pool).await.expect("insert order");
        id
    }
}
