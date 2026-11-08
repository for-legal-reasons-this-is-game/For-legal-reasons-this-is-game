// Pure planning of one batch: no I/O. Given the events and what Postgres knows about them, it
// decides which TigerBeetle transfers to send, which trades to insert and which orders to update.
// Everything here is safe to run on a replayed event: the database and tigerbeetle reject repeats.
use crate::consumer::ids::{refund_transfer_id, trade_leg_transfer_id};
use crate::domain::TransferCodeType;
use cn_tigerbeetle as tb;
use proto::{EventItem, ExecutionReport, OrderStatus, Side, TradeSettlement, event_item::Body};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OrderRow {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub market_id: i32,
    pub base_account_id: Uuid,
    pub quote_account_id: Uuid,
    pub buyer: bool,
    pub limit_price: Option<u128>,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

#[derive(Debug, Clone)]
pub struct MarketRow {
    pub market_id: i32,
    pub base_ledger_id: i32,
    pub quote_ledger_id: i32,
    // the only account the fees of this market may go to
    pub fee_account_id: Uuid,
}

#[derive(Debug, Default)]
pub struct Lookups {
    pub orders: HashMap<Uuid, OrderRow>,
    pub markets: HashMap<String, MarketRow>,
    pub known_trades: HashSet<Uuid>,
}

#[derive(Debug, Clone)]
pub struct TradeInsert {
    pub trade_id: Uuid,
    pub market_id: i32,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub price: u128,
    pub base_quantity: u128,
    pub quote_quantity: u128,
    pub buyer_fee: u128,
    pub seller_fee: u128,
    pub taker_side: i16,
    pub engine_time: SystemTime,
}

#[derive(Debug, Clone)]
pub struct OrderUpdate {
    pub order_id: Uuid,
    pub status: i16,
    pub cancel_reason: Option<i16>,
    pub filled_base: u128,
    pub filled_quote: u128,
    pub limit_price: Option<u128>,
}

#[derive(Debug, Default)]
pub struct BatchPlan {
    pub transfers: Vec<tb::Transfer>,
    pub trades: Vec<TradeInsert>,
    pub order_updates: Vec<OrderUpdate>,
}

#[derive(Debug)]
pub enum PlanError {
    Validation(String),
    UnknownOrder(Uuid),
    UnknownMarket(String),
    Malformed(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::Validation(m) => write!(f, "trade failed validation: {m}"),
            PlanError::UnknownOrder(o) => write!(f, "order {o} is not in postgres"),
            PlanError::UnknownMarket(s) => write!(f, "market {s} is not in postgres"),
            PlanError::Malformed(m) => write!(f, "malformed event: {m}"),
        }
    }
}

impl std::error::Error for PlanError {}

const TERMINAL: [OrderStatus; 3] = [
    OrderStatus::Filled,
    OrderStatus::Cancelled,
    OrderStatus::Expired,
];

fn id_of(v: &Option<proto::Id>, what: &str) -> Result<u128, PlanError> {
    v.map(u128::from)
        .ok_or_else(|| PlanError::Malformed(format!("{what} is missing")))
}

fn uuid_of(v: &Option<proto::Id>, what: &str) -> Result<Uuid, PlanError> {
    id_of(v, what).map(Uuid::from_u128)
}

fn amount_of(v: &Option<proto::Uint128>, what: &str) -> Result<u128, PlanError> {
    v.map(u128::from)
        .ok_or_else(|| PlanError::Malformed(format!("{what} is missing")))
}

fn push_unique<T: Clone + Eq + std::hash::Hash>(list: &mut Vec<T>, seen: &mut HashSet<T>, v: T) {
    if seen.insert(v.clone()) {
        list.push(v);
    }
}

// what to ask postgres for before planning: trade ids, order ids, market symbols of the batch
pub fn wanted_ids(items: &[EventItem]) -> (Vec<Uuid>, Vec<Uuid>, Vec<String>) {
    let (mut trades, mut orders, mut symbols) = (vec![], vec![], vec![]);
    let (mut seen_t, mut seen_o, mut seen_s) = (HashSet::new(), HashSet::new(), HashSet::new());
    for item in items {
        match &item.body {
            Some(Body::TradeSettlement(ts)) => {
                if let Some(id) = ts.trade_id {
                    push_unique(&mut trades, &mut seen_t, Uuid::from_u128(id.into()));
                }
                for id in [ts.buyer_order_id, ts.seller_order_id]
                    .into_iter()
                    .flatten()
                {
                    push_unique(&mut orders, &mut seen_o, Uuid::from_u128(id.into()));
                }
            }
            Some(Body::ExecutionReport(r)) => {
                if let Some(id) = r.order_id {
                    push_unique(&mut orders, &mut seen_o, Uuid::from_u128(id.into()));
                }
            }
            None => continue,
        }
        push_unique(&mut symbols, &mut seen_s, item.market_symbol.clone());
    }
    (trades, orders, symbols)
}

// How far an order has got, as a sort key: (finished?, filled base, filled quote). Of two reports
// of one order, the one with the bigger key is the newer.
type Progress = (bool, u128, u128);

// The newest limit price seen for each order that is still open. The consumer creates it once and
// passes it to every batch.
//
// A modify can reprice an order into the book, so the trade that follows its REPLACED report must
// be checked against the new limit. Postgres only learns the new limit when the batch with the
// report is saved, and the next batch is prepared before that, so the consumer remembers it itself.
// A report only replaces what is remembered if it has at least as much progress, so a replayed
// older report cannot bring an old limit back. A finished order is forgotten, which keeps this small.
#[derive(Default)]
pub struct LimitMemory {
    newest: HashMap<Uuid, (Progress, u128)>,
}

impl LimitMemory {
    fn get(&self, order_id: &Uuid) -> Option<u128> {
        self.newest.get(order_id).map(|(_, limit)| *limit)
    }

    fn see(&mut self, order_id: Uuid, progress: Progress, limit: Option<u128>) {
        let Some(limit) = limit else { return };
        match self.newest.get(&order_id) {
            Some((known, _)) if *known > progress => {}
            _ => {
                self.newest.insert(order_id, (progress, limit));
            }
        }
    }

    fn forget(&mut self, order_id: &Uuid) {
        self.newest.remove(order_id);
    }

    pub fn is_empty(&self) -> bool {
        self.newest.is_empty()
    }
}

// what the reports of one batch have decided so far
#[derive(Default)]
struct ReportState {
    refunded: HashSet<Uuid>,
    updates: HashMap<Uuid, OrderUpdate>,
    update_order: Vec<Uuid>,
}

pub fn build_plan(
    items: &[EventItem],
    lookups: &Lookups,
    memory: &mut LimitMemory,
) -> Result<BatchPlan, PlanError> {
    let mut plan = BatchPlan::default();
    let mut seen_trades = HashSet::new();
    let mut state = ReportState::default();

    for item in items {
        match &item.body {
            Some(Body::TradeSettlement(ts)) => {
                plan_trade(item, ts, lookups, memory, &mut seen_trades, &mut plan)?
            }
            Some(Body::ExecutionReport(r)) => {
                plan_report(item, r, lookups, memory, &mut state, &mut plan)?
            }
            None => eprintln!(
                "CONSUMER: event {} of {} has no body, skipping it",
                item.sequence, item.market_symbol
            ),
        }
    }
    plan.order_updates = state
        .update_order
        .into_iter()
        .filter_map(|id| state.updates.remove(&id))
        .collect();
    Ok(plan)
}

fn plan_trade(
    item: &EventItem,
    ts: &TradeSettlement,
    l: &Lookups,
    memory: &LimitMemory,
    seen: &mut HashSet<Uuid>,
    plan: &mut BatchPlan,
) -> Result<(), PlanError> {
    let trade_id = uuid_of(&ts.trade_id, "trade_id")?;
    // a trade we already recorded is never validated: its orders may have ended long ago
    if l.known_trades.contains(&trade_id) || !seen.insert(trade_id) {
        return Ok(());
    }
    let market = l
        .markets
        .get(&item.market_symbol)
        .ok_or_else(|| PlanError::UnknownMarket(item.market_symbol.clone()))?;
    let buyer_id = uuid_of(&ts.buyer_order_id, "buyer_order_id")?;
    let seller_id = uuid_of(&ts.seller_order_id, "seller_order_id")?;
    let buyer = l
        .orders
        .get(&buyer_id)
        .ok_or(PlanError::UnknownOrder(buyer_id))?;
    let seller = l
        .orders
        .get(&seller_id)
        .ok_or(PlanError::UnknownOrder(seller_id))?;

    let price = amount_of(&ts.price, "price")?;
    let base = amount_of(&ts.base_quantity, "base_quantity")?;
    let quote = amount_of(&ts.quote_quantity, "quote_quantity")?;
    // postgres refuses a trade with any of these at zero, after the money has moved; stop before
    if price == 0 || base == 0 || quote == 0 {
        return Err(PlanError::Validation(format!(
            "trade {trade_id}: price, base quantity and quote quantity must be above zero"
        )));
    }
    let buyer_fee = amount_of(&ts.buyer_fee, "buyer_fee")?;
    let seller_fee = amount_of(&ts.seller_fee, "seller_fee")?;
    let buyer_base = id_of(&ts.buyer_base_account, "buyer_base_account")?;
    let buyer_quote = id_of(&ts.buyer_quote_account, "buyer_quote_account")?;
    let seller_base = id_of(&ts.seller_base_account, "seller_base_account")?;
    let seller_quote = id_of(&ts.seller_quote_account, "seller_quote_account")?;
    let fee_account = id_of(&ts.fee_account, "fee_account")?;
    let taker = match Side::try_from(ts.taker_side) {
        Ok(s @ (Side::Buy | Side::Sell)) => s,
        _ => {
            return Err(PlanError::Malformed(format!(
                "taker_side {}",
                ts.taker_side
            )));
        }
    };
    let engine_time = item
        .engine_time
        .ok_or_else(|| PlanError::Malformed("engine_time is missing".into()))
        .and_then(|t| {
            SystemTime::try_from(t).map_err(|e| PlanError::Malformed(format!("engine_time: {e}")))
        })?;

    // validation, against what the backend wrote itself
    let fail = |m: String| Err(PlanError::Validation(format!("trade {trade_id}: {m}")));
    for (side, order) in [("buyer", buyer), ("seller", seller)] {
        if order.market_id != market.market_id {
            return fail(format!("{side} order is in another market"));
        }
    }
    if uuid_of(&ts.buyer_account_holder, "buyer_account_holder")? != buyer.user_id
        || uuid_of(&ts.seller_account_holder, "seller_account_holder")? != seller.user_id
    {
        return fail("account holder does not match the order's user".into());
    }
    if Uuid::from_u128(buyer_base) != buyer.base_account_id
        || Uuid::from_u128(buyer_quote) != buyer.quote_account_id
        || Uuid::from_u128(seller_base) != seller.base_account_id
        || Uuid::from_u128(seller_quote) != seller.quote_account_id
    {
        return fail("an account does not belong to its order".into());
    }
    if Uuid::from_u128(fee_account) != market.fee_account_id {
        return fail("fee account is not the market's fee account".into());
    }
    let limit_of = |order: &OrderRow| memory.get(&order.order_id).or(order.limit_price);
    if limit_of(buyer).is_some_and(|limit| price > limit) {
        return fail(format!("price {price} is above the buyer's limit"));
    }
    if limit_of(seller).is_some_and(|limit| price < limit) {
        return fail(format!("price {price} is below the seller's limit"));
    }
    let bps = |order: &OrderRow, taker_order: bool| {
        if taker_order {
            order.taker_fee_bps
        } else {
            order.maker_fee_bps
        }
    };
    if buyer_fee > fee_cap(quote, bps(buyer, taker == Side::Buy)) {
        return fail("buyer fee is above the cap of its fee rate".into());
    }
    if seller_fee > fee_cap(quote, bps(seller, taker == Side::Sell)) {
        return fail("seller fee is above the cap of its fee rate".into());
    }

    // the legs of one trade are one linked chain, in this order
    let base_ledger = market.base_ledger_id as u32;
    let quote_ledger = market.quote_ledger_id as u32;
    let (buyer_hold, seller_hold) = (buyer_id.as_u128(), seller_id.as_u128());
    let mut legs = vec![
        (
            1u8,
            seller_hold,
            buyer_base,
            base,
            base_ledger,
            TransferCodeType::Trade,
        ),
        (
            2,
            buyer_hold,
            seller_quote,
            quote,
            quote_ledger,
            TransferCodeType::Trade,
        ),
    ];
    if buyer_fee > 0 {
        legs.push((
            3,
            buyer_hold,
            fee_account,
            buyer_fee,
            quote_ledger,
            TransferCodeType::FeeCollection,
        ));
    }
    if seller_fee > 0 {
        legs.push((
            4,
            seller_quote,
            fee_account,
            seller_fee,
            quote_ledger,
            TransferCodeType::FeeCollection,
        ));
    }
    let last = legs.len() - 1;
    for (i, (leg, debit, credit, amount, ledger, code)) in legs.into_iter().enumerate() {
        plan.transfers.push(tb::Transfer {
            id: trade_leg_transfer_id(trade_id.as_u128(), leg),
            debit_account_id: debit,
            credit_account_id: credit,
            amount,
            ledger,
            code: code as u16,
            flags: if i < last {
                tb::TransferFlags::Linked
            } else {
                tb::TransferFlags::empty()
            },
            ..Default::default()
        });
    }
    plan.trades.push(TradeInsert {
        trade_id,
        market_id: market.market_id,
        buyer_order_id: buyer_id,
        seller_order_id: seller_id,
        price,
        base_quantity: base,
        quote_quantity: quote,
        buyer_fee,
        seller_fee,
        taker_side: taker as i16,
        engine_time,
    });
    Ok(())
}

// ceil(quote * bps / 10000) without a 256-bit intermediate; saturates instead of overflowing
fn fee_cap(quote: u128, bps: i32) -> u128 {
    let bps = bps.max(0) as u128;
    let whole = (quote / 10_000).saturating_mul(bps);
    let rest = (quote % 10_000) * bps;
    whole.saturating_add(rest.div_ceil(10_000))
}

fn plan_report(
    item: &EventItem,
    r: &ExecutionReport,
    l: &Lookups,
    memory: &mut LimitMemory,
    state: &mut ReportState,
    plan: &mut BatchPlan,
) -> Result<(), PlanError> {
    let order_id = uuid_of(&r.order_id, "order_id")?;
    let order = l
        .orders
        .get(&order_id)
        .ok_or(PlanError::UnknownOrder(order_id))?;
    let status = match OrderStatus::try_from(r.status) {
        Ok(OrderStatus::Unspecified) | Err(_) => {
            return Err(PlanError::Malformed(format!("order status {}", r.status)));
        }
        Ok(s) => s,
    };
    let cancel_reason = match r.cancel_reason {
        0 => None,
        v => {
            Some(i16::try_from(v).map_err(|_| PlanError::Malformed(format!("cancel_reason {v}")))?)
        }
    };
    let update = OrderUpdate {
        order_id,
        status: status as i16,
        cancel_reason,
        filled_base: amount_of(&r.filled_base_quantity, "filled_base_quantity")?,
        filled_quote: amount_of(&r.filled_quote_quantity, "filled_quote_quantity")?,
        limit_price: r.limit_price.map(u128::from),
    };
    let terminal = TERMINAL.contains(&status);

    // one update per order: the report with the most progress wins, the later one on a tie
    let rank = |u: &OrderUpdate, terminal: bool| (terminal, u.filled_base, u.filled_quote);

    // remember the new limit for the trades that follow (this batch or the next); forget it when
    // the order is over
    if terminal {
        memory.forget(&order_id);
    } else {
        memory.see(order_id, rank(&update, terminal), update.limit_price);
    }
    match state.updates.get(&order_id) {
        None => {
            state.update_order.push(order_id);
            state.updates.insert(order_id, update);
        }
        Some(old) => {
            let old_terminal = TERMINAL.iter().any(|s| *s as i16 == old.status);
            if rank(&update, terminal) >= rank(old, old_terminal) {
                state.updates.insert(order_id, update);
            }
        }
    }

    if terminal && state.refunded.insert(order_id) {
        let market = l
            .markets
            .get(&item.market_symbol)
            .ok_or_else(|| PlanError::UnknownMarket(item.market_symbol.clone()))?;
        // a buy reserved the quote asset, a sell the base asset
        let (credit, ledger) = if order.buyer {
            (order.quote_account_id, market.quote_ledger_id)
        } else {
            (order.base_account_id, market.base_ledger_id)
        };
        plan.transfers.push(tb::Transfer {
            id: refund_transfer_id(order_id.as_u128()),
            debit_account_id: order_id.as_u128(),
            credit_account_id: credit.as_u128(),
            amount: u128::MAX,
            ledger: ledger as u32,
            code: TransferCodeType::Refund as u16,
            flags: tb::TransferFlags::BalancingDebit,
            ..Default::default()
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cn_tigerbeetle as tb;
    use proto::{
        EventItem, ExecutionReport, Id, OrderStatus, Side, TradeSettlement, Uint128,
        event_item::Body,
    };

    use crate::consumer::ids::{refund_transfer_id, trade_leg_transfer_id};
    use crate::domain::TransferCodeType;

    // one batch on its own: the consumer's memory of limit prices starts empty
    fn build_plan(items: &[EventItem], l: &Lookups) -> Result<BatchPlan, PlanError> {
        super::build_plan(items, l, &mut LimitMemory::default())
    }

    // a batch planned with a memory that lives on between batches, like the consumer's
    fn plan_with(
        memory: &mut LimitMemory,
        items: &[EventItem],
        l: &Lookups,
    ) -> Result<BatchPlan, PlanError> {
        super::build_plan(items, l, memory)
    }

    const SYMBOL: &str = "BTC-USD";
    const BUYER: u128 = 0xB1;
    const SELLER: u128 = 0x51;
    const FEE_ACCOUNT: u128 = 0xFE;

    fn uid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }
    fn id(n: u128) -> Option<Id> {
        Some(Id::from(n))
    }
    fn amount(n: u128) -> Option<Uint128> {
        Some(Uint128::from(n))
    }

    // buyer: user 0xB0, base account 0xB2, quote account 0xB3; seller: user 0x50, 0x52, 0x53.
    // base ledger 1, quote ledger 2. Both orders pay maker 10 bps / taker 30 bps.
    fn lookups() -> Lookups {
        let order = |order: u128, user: u128, base: u128, quote: u128, buyer: bool, limit: u128| {
            (
                uid(order),
                OrderRow {
                    order_id: uid(order),
                    user_id: uid(user),
                    market_id: 7,
                    base_account_id: uid(base),
                    quote_account_id: uid(quote),
                    buyer,
                    limit_price: Some(limit),
                    maker_fee_bps: 10,
                    taker_fee_bps: 30,
                },
            )
        };
        Lookups {
            orders: [
                order(BUYER, 0xB0, 0xB2, 0xB3, true, 1100),
                order(SELLER, 0x50, 0x52, 0x53, false, 900),
            ]
            .into_iter()
            .collect(),
            markets: [(
                SYMBOL.to_string(),
                MarketRow {
                    market_id: 7,
                    base_ledger_id: 1,
                    quote_ledger_id: 2,
                    fee_account_id: uid(FEE_ACCOUNT),
                },
            )]
            .into_iter()
            .collect(),
            known_trades: Default::default(),
        }
    }

    struct T {
        trade: u128,
        price: u128,
        base: u128,
        quote: u128,
        buyer_fee: u128,
        seller_fee: u128,
        taker: Side,
    }

    // buyer is the taker: cap 3000 (30 bps of 1_000_000); seller is the maker: cap 1000 (10 bps)
    fn t(trade: u128) -> T {
        T {
            trade,
            price: 1000,
            base: 1000,
            quote: 1_000_000,
            buyer_fee: 3000,
            seller_fee: 1000,
            taker: Side::Buy,
        }
    }

    fn item(body: Body) -> EventItem {
        EventItem {
            sequence: 1,
            engine_time: Some(proto::Timestamp {
                seconds: 1_700_000_000,
                nanos: 5,
            }),
            market_symbol: SYMBOL.to_string(),
            body: Some(body),
        }
    }

    fn trade_item(t: &T) -> EventItem {
        item(Body::TradeSettlement(TradeSettlement {
            trade_id: id(t.trade),
            price: amount(t.price),
            base_quantity: amount(t.base),
            quote_quantity: amount(t.quote),
            taker_side: t.taker as i32,
            buyer_order_id: id(BUYER),
            buyer_account_holder: id(0xB0),
            buyer_base_account: id(0xB2),
            buyer_quote_account: id(0xB3),
            buyer_fee: amount(t.buyer_fee),
            seller_order_id: id(SELLER),
            seller_account_holder: id(0x50),
            seller_base_account: id(0x52),
            seller_quote_account: id(0x53),
            seller_fee: amount(t.seller_fee),
            fee_account: id(FEE_ACCOUNT),
        }))
    }

    fn report(
        order: u128,
        status: OrderStatus,
        filled_base: u128,
        filled_quote: u128,
    ) -> EventItem {
        item(Body::ExecutionReport(ExecutionReport {
            order_id: id(order),
            status: status as i32,
            filled_base_quantity: amount(filled_base),
            filled_quote_quantity: amount(filled_quote),
            limit_price: amount(1000),
            ..Default::default()
        }))
    }

    fn is_linked(t: &tb::Transfer) -> bool {
        t.flags.contains(tb::TransferFlags::Linked)
    }

    #[test]
    fn known_trade_is_skipped_without_validation() {
        let mut l = lookups();
        l.known_trades.insert(uid(1));
        l.orders.clear(); // validating it would fail
        let p = build_plan(&[trade_item(&t(1))], &l).unwrap();
        assert!(p.transfers.is_empty() && p.trades.is_empty());
    }

    #[test]
    fn duplicate_trade_in_one_batch_is_planned_once() {
        let p = build_plan(&[trade_item(&t(1)), trade_item(&t(1))], &lookups()).unwrap();
        assert_eq!(p.transfers.len(), 4);
        assert_eq!(p.trades.len(), 1);
    }

    #[test]
    fn new_trade_builds_four_linked_legs_in_order() {
        let p = build_plan(&[trade_item(&t(1))], &lookups()).unwrap();
        assert_eq!(p.transfers.len(), 4);
        let want = [
            // (debit, credit, amount, ledger, code)
            (SELLER, 0xB2, 1000, 1, TransferCodeType::Trade),
            (BUYER, 0x53, 1_000_000, 2, TransferCodeType::Trade),
            (BUYER, FEE_ACCOUNT, 3000, 2, TransferCodeType::FeeCollection),
            (0x53, FEE_ACCOUNT, 1000, 2, TransferCodeType::FeeCollection),
        ];
        for (i, (debit, credit, amount, ledger, code)) in want.into_iter().enumerate() {
            let tr = &p.transfers[i];
            assert_eq!(tr.id, trade_leg_transfer_id(1, i as u8 + 1), "leg {i} id");
            assert_eq!(tr.debit_account_id, debit, "leg {i} debit");
            assert_eq!(tr.credit_account_id, credit, "leg {i} credit");
            assert_eq!(tr.amount, amount, "leg {i} amount");
            assert_eq!(tr.ledger, ledger, "leg {i} ledger");
            assert_eq!(tr.code, code as u16, "leg {i} code");
            assert_eq!(is_linked(tr), i < 3, "leg {i} linked");
        }
        assert_eq!(p.trades.len(), 1);
        assert_eq!(p.trades[0].market_id, 7);
        assert_eq!(p.trades[0].buyer_order_id, uid(BUYER));
    }

    #[test]
    fn zero_fees_skip_legs_3_and_4_and_close_the_chain_on_leg_2() {
        let mut trade = t(1);
        trade.buyer_fee = 0;
        trade.seller_fee = 0;
        let p = build_plan(&[trade_item(&trade)], &lookups()).unwrap();
        assert_eq!(p.transfers.len(), 2);
        assert!(is_linked(&p.transfers[0]));
        assert!(!is_linked(&p.transfers[1]));
        assert_eq!(p.transfers[1].id, trade_leg_transfer_id(1, 2));
    }

    #[test]
    fn two_trades_in_one_batch_form_separate_chains() {
        let p = build_plan(&[trade_item(&t(1)), trade_item(&t(2))], &lookups()).unwrap();
        assert_eq!(p.transfers.len(), 8);
        assert!(!is_linked(&p.transfers[3]));
        assert!(!is_linked(&p.transfers[7]));
        assert!(is_linked(&p.transfers[4]));
    }

    #[test]
    fn price_above_buyer_limit_is_a_validation_error() {
        let mut trade = t(1);
        trade.price = 1200;
        let err = build_plan(&[trade_item(&trade)], &lookups()).unwrap_err();
        assert!(matches!(err, PlanError::Validation(_)), "{err:?}");
    }

    #[test]
    fn price_below_seller_limit_is_a_validation_error() {
        let mut trade = t(1);
        trade.price = 800;
        let err = build_plan(&[trade_item(&trade)], &lookups()).unwrap_err();
        assert!(matches!(err, PlanError::Validation(_)), "{err:?}");
    }

    #[test]
    fn market_orders_have_no_limit_to_violate() {
        let mut l = lookups();
        for o in l.orders.values_mut() {
            o.limit_price = None;
        }
        let mut trade = t(1);
        trade.price = 5000;
        assert!(build_plan(&[trade_item(&trade)], &l).is_ok());
    }

    #[test]
    fn fee_above_cap_is_a_validation_error() {
        let mut trade = t(1);
        trade.buyer_fee = 3001;
        assert!(matches!(
            build_plan(&[trade_item(&trade)], &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
        let mut trade = t(1);
        trade.seller_fee = 1001;
        assert!(matches!(
            build_plan(&[trade_item(&trade)], &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
    }

    #[test]
    fn account_that_does_not_belong_to_the_order_is_a_validation_error() {
        let mut it = trade_item(&t(1));
        if let Some(Body::TradeSettlement(ts)) = it.body.as_mut() {
            ts.buyer_base_account = id(0xDEAD);
        }
        assert!(matches!(
            build_plan(&[it], &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
    }

    #[test]
    fn unknown_order_and_unknown_market_are_errors() {
        let mut l = lookups();
        l.orders.remove(&uid(SELLER));
        assert!(matches!(
            build_plan(&[trade_item(&t(1))], &l).unwrap_err(),
            PlanError::UnknownOrder(o) if o == uid(SELLER)
        ));
        let mut it = trade_item(&t(1));
        it.market_symbol = "NOPE".to_string();
        assert!(matches!(
            build_plan(&[it], &lookups()).unwrap_err(),
            PlanError::UnknownMarket(s) if s == "NOPE"
        ));
    }

    #[test]
    fn terminal_report_builds_one_balancing_refund_from_the_reserved_account() {
        let p = build_plan(
            &[report(BUYER, OrderStatus::Filled, 1000, 1_000_000)],
            &lookups(),
        )
        .unwrap();
        assert_eq!(p.transfers.len(), 1);
        let r = &p.transfers[0];
        assert_eq!(r.id, refund_transfer_id(BUYER));
        assert_eq!(r.debit_account_id, BUYER); // the hold account is the order id
        assert_eq!(r.credit_account_id, 0xB3); // a buy reserves the quote asset
        assert_eq!(r.ledger, 2);
        assert_eq!(r.amount, u128::MAX);
        assert_eq!(r.code, TransferCodeType::Refund as u16);
        assert!(r.flags.contains(tb::TransferFlags::BalancingDebit));
        assert!(!is_linked(r));

        let p = build_plan(&[report(SELLER, OrderStatus::Cancelled, 0, 0)], &lookups()).unwrap();
        assert_eq!(p.transfers[0].credit_account_id, 0x52); // a sell reserves the base asset
        assert_eq!(p.transfers[0].ledger, 1);
    }

    #[test]
    fn terminal_refund_is_emitted_once_even_with_duplicate_terminal_reports() {
        let r = report(BUYER, OrderStatus::Filled, 1000, 1_000_000);
        let p = build_plan(&[r.clone(), r], &lookups()).unwrap();
        assert_eq!(p.transfers.len(), 1);
    }

    #[test]
    fn non_terminal_report_builds_no_refund() {
        for s in [
            OrderStatus::New,
            OrderStatus::PartiallyFilled,
            OrderStatus::PendingTrigger,
        ] {
            let p = build_plan(&[report(BUYER, s, 1, 1)], &lookups()).unwrap();
            assert!(p.transfers.is_empty(), "{s:?}");
            assert_eq!(p.order_updates.len(), 1);
        }
    }

    #[test]
    fn two_reports_for_one_order_collapse_to_the_highest_filled() {
        let a = report(BUYER, OrderStatus::PartiallyFilled, 2, 2000);
        let b = report(BUYER, OrderStatus::PartiallyFilled, 5, 5000);
        let p = build_plan(&[a.clone(), b.clone()], &lookups()).unwrap();
        assert_eq!(p.order_updates.len(), 1);
        assert_eq!(p.order_updates[0].filled_base, 5);
        // a replay arriving after live items must not win
        let p = build_plan(&[b, a], &lookups()).unwrap();
        assert_eq!(p.order_updates.len(), 1);
        assert_eq!(p.order_updates[0].filled_base, 5);
    }

    #[test]
    fn terminal_beats_non_terminal_and_later_wins_ties() {
        let filled = report(BUYER, OrderStatus::Filled, 5, 5000);
        let partial = report(BUYER, OrderStatus::PartiallyFilled, 5, 5000);
        let p = build_plan(&[filled, partial], &lookups()).unwrap();
        assert_eq!(p.order_updates[0].status, OrderStatus::Filled as i16);

        let new = report(BUYER, OrderStatus::New, 0, 0);
        let mut replaced = report(BUYER, OrderStatus::New, 0, 0);
        if let Some(Body::ExecutionReport(r)) = replaced.body.as_mut() {
            r.limit_price = amount(1050);
        }
        let p = build_plan(&[new, replaced], &lookups()).unwrap();
        assert_eq!(p.order_updates[0].limit_price, Some(1050));
    }

    #[test]
    fn report_for_an_order_missing_from_postgres_is_an_error() {
        let err = build_plan(&[report(999, OrderStatus::New, 0, 0)], &lookups()).unwrap_err();
        assert!(
            matches!(err, PlanError::UnknownOrder(o) if o == uid(999)),
            "{err:?}"
        );
    }

    #[test]
    fn item_without_a_body_is_skipped() {
        let it = EventItem {
            market_symbol: SYMBOL.to_string(),
            ..Default::default()
        };
        let p = build_plan(&[it], &lookups()).unwrap();
        assert!(p.transfers.is_empty() && p.trades.is_empty() && p.order_updates.is_empty());
    }

    #[test]
    fn unspecified_status_is_malformed() {
        let err =
            build_plan(&[report(BUYER, OrderStatus::Unspecified, 0, 0)], &lookups()).unwrap_err();
        assert!(matches!(err, PlanError::Malformed(_)), "{err:?}");
    }

    #[test]
    fn wanted_ids_lists_trades_orders_and_symbols() {
        let items = [trade_item(&t(1)), report(BUYER, OrderStatus::New, 0, 0)];
        let (trades, orders, symbols) = wanted_ids(&items);
        assert_eq!(trades, vec![uid(1)]);
        assert!(orders.contains(&uid(BUYER)) && orders.contains(&uid(SELLER)));
        assert_eq!(symbols, vec![SYMBOL.to_string()]);
    }

    // --- review fixes ---------------------------------------------------------------------------

    fn repricing_report(order: u128, new_limit: u128) -> EventItem {
        let mut it = report(order, OrderStatus::New, 0, 0);
        if let Some(Body::ExecutionReport(r)) = it.body.as_mut() {
            r.limit_price = amount(new_limit);
        }
        it
    }

    #[test]
    fn trade_after_a_repricing_report_in_the_same_batch_uses_the_new_limit() {
        // the buyer's stored limit is 1100; a modify raised it to 1300 and it matched at 1200
        let mut trade = t(1);
        trade.price = 1200;
        let items = [repricing_report(BUYER, 1300), trade_item(&trade)];
        assert!(build_plan(&items, &lookups()).is_ok());
    }

    #[test]
    fn trade_before_the_repricing_report_still_uses_the_stored_limit() {
        let mut trade = t(1);
        trade.price = 1200;
        let items = [trade_item(&trade), repricing_report(BUYER, 1300)];
        assert!(matches!(
            build_plan(&items, &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
    }

    #[test]
    fn a_replayed_older_report_does_not_lower_the_limit_used_for_later_trades() {
        let mut trade = t(1);
        trade.price = 1200;
        // newer repricing (filled 5), then a replayed older report (filled 2) with the old limit
        let mut newer = repricing_report(BUYER, 1300);
        let mut older = repricing_report(BUYER, 1100);
        if let (Some(Body::ExecutionReport(n)), Some(Body::ExecutionReport(o))) =
            (newer.body.as_mut(), older.body.as_mut())
        {
            n.filled_base_quantity = amount(5);
            o.filled_base_quantity = amount(2);
        }
        assert!(build_plan(&[newer, older, trade_item(&trade)], &lookups()).is_ok());
    }

    #[test]
    fn zero_price_or_quantity_is_a_validation_error() {
        for mutate in [
            (|t: &mut T| t.price = 0) as fn(&mut T),
            |t: &mut T| t.base = 0,
            |t: &mut T| t.quote = 0,
        ] {
            let mut trade = t(1);
            mutate(&mut trade);
            assert!(matches!(
                build_plan(&[trade_item(&trade)], &lookups()).unwrap_err(),
                PlanError::Validation(_)
            ));
        }
    }

    #[test]
    fn fee_account_other_than_the_markets_is_a_validation_error() {
        let mut it = trade_item(&t(1));
        if let Some(Body::TradeSettlement(ts)) = it.body.as_mut() {
            ts.fee_account = id(0x53); // the seller's own quote account
        }
        assert!(matches!(
            build_plan(&[it], &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
    }

    // --- limit prices remembered across batches ------------------------------------------------

    #[test]
    fn a_repricing_in_an_earlier_batch_is_remembered() {
        let mut memory = LimitMemory::default();
        let mut trade = t(1);
        trade.price = 1200;
        // batch 1 ends with the repricing, batch 2 starts with the trade; postgres still has 1100
        plan_with(&mut memory, &[repricing_report(BUYER, 1300)], &lookups()).unwrap();
        assert!(plan_with(&mut memory, &[trade_item(&trade)], &lookups()).is_ok());
    }

    #[test]
    fn a_replayed_older_report_in_a_later_batch_does_not_lower_the_limit() {
        let mut memory = LimitMemory::default();
        let mut newer = repricing_report(BUYER, 1300);
        let mut older = repricing_report(BUYER, 1100);
        if let (Some(Body::ExecutionReport(n)), Some(Body::ExecutionReport(o))) =
            (newer.body.as_mut(), older.body.as_mut())
        {
            n.filled_base_quantity = amount(5);
            o.filled_base_quantity = amount(2);
        }
        let mut trade = t(1);
        trade.price = 1200;
        plan_with(&mut memory, &[newer], &lookups()).unwrap();
        plan_with(&mut memory, &[older], &lookups()).unwrap();
        assert!(plan_with(&mut memory, &[trade_item(&trade)], &lookups()).is_ok());
    }

    #[test]
    fn a_finished_order_is_forgotten() {
        let mut memory = LimitMemory::default();
        let finished = report(BUYER, OrderStatus::Filled, 1000, 1_000_000);
        plan_with(
            &mut memory,
            &[repricing_report(BUYER, 1300), finished],
            &lookups(),
        )
        .unwrap();
        assert!(memory.is_empty());
        // forgotten: the stored limit (1100) applies again
        let mut trade = t(1);
        trade.price = 1200;
        assert!(matches!(
            plan_with(&mut memory, &[trade_item(&trade)], &lookups()).unwrap_err(),
            PlanError::Validation(_)
        ));
    }
}
