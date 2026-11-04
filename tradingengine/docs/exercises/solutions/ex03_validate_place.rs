#![allow(unused)]
// ex03: validate a PlaceOrderRequest in the contract's order
//
// Sprint: PR A, steps 2 and 3. The tests are the ones SPRINT.md asks for.
// Read: proto/how_to_use.md "Validating a place command" and DATA_MODEL.md
// "Validation". The order of the checks matters: a request with two mistakes
// gets the reason of the first check it fails.
//
// The order used here:
//   0. ids present (command_id first, the cache needs it)  BAD_ORDER_PARAMS
//   1. market exists                                        UNKNOWN_MARKET
//   -  side, type, time_in_force known (needed by 2)        BAD_ORDER_PARAMS
//   2. right amount arm (quote only for market/stop-market buys)  BAD_ORDER_PARAMS
//   3. quantity above zero                                  BAD_QUANTITY
//   4. every present price above zero                       BAD_PRICE
//   5. limit_price set exactly for LIMIT and STOP_LIMIT     BAD_ORDER_PARAMS
//   6. trigger_price + trigger_direction exactly for stops  BAD_ORDER_PARAMS
//   7. expire_time exactly for GTD, and in the future       BAD_ORDER_PARAMS
//   8. protection_price only on market orders               BAD_ORDER_PARAMS
//   9. the quote fits in 128 bits after the division        BAD_QUANTITY
//  10. not LIMIT + GTC: not built yet                       UNSUPPORTED
//
// You may copy your ex01 conversions in.

use std::collections::HashMap;
use std::time::SystemTime;

use proto::place_order_request::Amount;
use proto::{OrderType, PlaceOrderRequest, RejectReason, TimeInForce, TriggerDirection};

use engine_sprint::types::LimitGtc;
use tradingengine::ids::{CommandId, OrderId, UserId};
use tradingengine::market::MarketInfo;
use tradingengine::order::Side;
use tradingengine::price::Price;
use tradingengine::quantity::BaseQuantity;
use tradingengine::wire::required_id;

pub fn validate(
    request: PlaceOrderRequest,
    markets: &HashMap<String, MarketInfo>,
    now: SystemTime,
) -> Result<LimitGtc, RejectReason> {
    //>>
    use RejectReason::{BadOrderParams, BadPrice, BadQuantity, Unsupported};

    let id = |field| required_id(field).map_err(|_| BadOrderParams);
    let command_id = CommandId::new(id(request.command_id)?);
    let order_id = OrderId::new(id(request.order_id)?);
    let account_holder = UserId::new(id(request.account_holder)?);
    id(request.base_account)?;
    id(request.quote_account)?;

    let market = markets
        .get(&request.market_symbol)
        .ok_or(RejectReason::UnknownMarket)?;

    let side = match proto::Side::try_from(request.side) {
        Ok(proto::Side::Buy) => Side::Buy,
        Ok(proto::Side::Sell) => Side::Sell,
        _ => return Err(BadOrderParams),
    };
    let order_type = match OrderType::try_from(request.r#type) {
        Ok(OrderType::Unspecified) | Err(_) => return Err(BadOrderParams),
        Ok(order_type) => order_type,
    };
    let time_in_force = match TimeInForce::try_from(request.time_in_force) {
        Ok(TimeInForce::Unspecified) | Err(_) => return Err(BadOrderParams),
        Ok(time_in_force) => time_in_force,
    };

    let is_market = matches!(order_type, OrderType::Market | OrderType::StopMarket);
    let is_stop = matches!(order_type, OrderType::StopLimit | OrderType::StopMarket);
    let needs_limit = matches!(order_type, OrderType::Limit | OrderType::StopLimit);

    // 2. the amount arm
    let amount = match (request.amount, is_market && side == Side::Buy) {
        (Some(Amount::QuoteQuantity(quote)), true) => quote,
        (Some(Amount::BaseQuantity(base)), false) => base,
        _ => return Err(BadOrderParams),
    };

    // 3. quantity
    if u128::from(amount) == 0 {
        return Err(BadQuantity);
    }

    // 4. every price that is there
    let prices = [
        request.limit_price,
        request.trigger_price,
        request.protection_price,
    ];
    if prices.into_iter().flatten().any(|p| u128::from(p) == 0) {
        return Err(BadPrice);
    }

    // 5. limit price
    if request.limit_price.is_some() != needs_limit {
        return Err(BadOrderParams);
    }

    // 6. trigger price and direction
    let has_direction = match TriggerDirection::try_from(request.trigger_direction) {
        Ok(TriggerDirection::Unspecified) => false,
        Ok(_) => true,
        Err(_) => return Err(BadOrderParams),
    };
    if request.trigger_price.is_some() != is_stop || has_direction != is_stop {
        return Err(BadOrderParams);
    }

    // 7. expire time
    match (request.expire_time, time_in_force == TimeInForce::Gtd) {
        (None, false) => {}
        (Some(expire), true) => {
            let expire = SystemTime::try_from(expire).map_err(|_| BadOrderParams)?;
            if expire <= now {
                return Err(BadOrderParams);
            }
        }
        _ => return Err(BadOrderParams),
    }

    // 8. protection price
    if request.protection_price.is_some() && !is_market {
        return Err(BadOrderParams);
    }

    // 9. the quote fits, only checkable with a limit price and a base amount
    let base = BaseQuantity::from(amount);
    let price = match request.limit_price {
        Some(limit) => Some(Price::try_from(limit).map_err(|_| BadPrice)?),
        None => None,
    };
    if let (Some(price), false) = (price, is_market && side == Side::Buy) {
        market
            .scale()
            .quote_for(price, base)
            .map_err(|_| BadQuantity)?;
    }

    // 10. only limit GTC is built
    let (OrderType::Limit, TimeInForce::Gtc, Some(price)) = (order_type, time_in_force, price)
    else {
        return Err(Unsupported);
    };

    Ok(LimitGtc {
        command_id,
        order_id,
        symbol: request.market_symbol,
        account_holder,
        side,
        price,
        quantity: base,
    })
    //<<
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use engine_sprint::fixtures::{BTC, BTC_USD, btc_usd, dollars, limit_request};
    use proto::{Timestamp, Uint128};

    fn markets() -> HashMap<String, MarketInfo> {
        HashMap::from([(BTC_USD.to_string(), MarketInfo::new(btc_usd()).unwrap())])
    }

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000)
    }

    fn check(request: PlaceOrderRequest) -> Result<LimitGtc, RejectReason> {
        validate(request, &markets(), now())
    }

    fn buy() -> PlaceOrderRequest {
        limit_request(proto::Side::Buy, dollars(29_000), BTC / 2)
    }

    fn wire(v: u128) -> Option<Uint128> {
        Some(Uint128::from(v))
    }

    fn at(secs_from_now: i64) -> Option<Timestamp> {
        Some(Timestamp {
            seconds: 1_800_000_000 + secs_from_now,
            nanos: 0,
        })
    }

    #[test]
    fn a_plain_limit_gtc_is_accepted() {
        let parsed = check(buy()).expect("valid");
        assert_eq!(parsed.side, Side::Buy);
        assert_eq!(parsed.price, Price::from_minor_units(dollars(29_000)).unwrap());
        assert_eq!(parsed.quantity, BaseQuantity::from_minor_units(BTC / 2));
        assert_eq!(parsed.order_id, OrderId::new(200));
        assert_eq!(parsed.symbol, BTC_USD);
    }

    #[test]
    fn unknown_market() {
        let request = PlaceOrderRequest {
            market_symbol: "SOL-USD".into(),
            ..buy()
        };
        assert_eq!(check(request), Err(RejectReason::UnknownMarket));

        let empty = PlaceOrderRequest {
            market_symbol: String::new(),
            ..buy()
        };
        assert_eq!(check(empty), Err(RejectReason::UnknownMarket));
    }

    #[test]
    fn zero_quantity_and_zero_price() {
        let zero_qty = limit_request(proto::Side::Buy, dollars(29_000), 0);
        assert_eq!(check(zero_qty), Err(RejectReason::BadQuantity));

        let zero_price = limit_request(proto::Side::Buy, 0, BTC);
        assert_eq!(check(zero_price), Err(RejectReason::BadPrice));
    }

    #[test]
    fn the_first_failing_check_wins() {
        // unknown market beats a zero quantity
        let request = PlaceOrderRequest {
            market_symbol: "SOL-USD".into(),
            ..limit_request(proto::Side::Buy, dollars(29_000), 0)
        };
        assert_eq!(check(request), Err(RejectReason::UnknownMarket));

        // a zero quantity beats a zero price
        let request = limit_request(proto::Side::Buy, 0, 0);
        assert_eq!(check(request), Err(RejectReason::BadQuantity));
    }

    #[test]
    fn missing_ids_and_enums_are_bad_order_params() {
        let no_order_id = PlaceOrderRequest {
            order_id: None,
            ..buy()
        };
        assert_eq!(check(no_order_id), Err(RejectReason::BadOrderParams));

        let no_side = PlaceOrderRequest { side: 0, ..buy() };
        assert_eq!(check(no_side), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn a_market_buy_must_say_how_much_to_spend() {
        let base_arm = PlaceOrderRequest {
            r#type: OrderType::Market as i32,
            limit_price: None,
            ..buy()
        };
        assert_eq!(check(base_arm.clone()), Err(RejectReason::BadOrderParams));

        let quote_arm = PlaceOrderRequest {
            amount: Some(Amount::QuoteQuantity(Uint128::from(100_000u128))),
            ..base_arm
        };
        // a valid market buy, but not built in this slice
        assert_eq!(check(quote_arm), Err(RejectReason::Unsupported));
    }

    #[test]
    fn a_limit_order_needs_its_price() {
        let request = PlaceOrderRequest {
            limit_price: None,
            ..buy()
        };
        assert_eq!(check(request), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn protection_price_is_for_market_orders_only() {
        let request = PlaceOrderRequest {
            protection_price: wire(dollars(30_000)),
            ..buy()
        };
        assert_eq!(check(request), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn a_zero_trigger_price_is_a_bad_price_even_on_a_limit() {
        let request = PlaceOrderRequest {
            trigger_price: wire(0),
            ..buy()
        };
        assert_eq!(check(request), Err(RejectReason::BadPrice));
    }

    #[test]
    fn stops_need_trigger_fields_and_are_unsupported() {
        let stop = PlaceOrderRequest {
            r#type: OrderType::StopLimit as i32,
            ..buy()
        };
        assert_eq!(check(stop.clone()), Err(RejectReason::BadOrderParams));

        let complete = PlaceOrderRequest {
            trigger_price: wire(dollars(28_000)),
            trigger_direction: TriggerDirection::LastAtOrBelow as i32,
            ..stop
        };
        assert_eq!(check(complete), Err(RejectReason::Unsupported));
    }

    #[test]
    fn expire_time_goes_with_gtd_only() {
        let gtc_with_expiry = PlaceOrderRequest {
            expire_time: at(60),
            ..buy()
        };
        assert_eq!(check(gtc_with_expiry), Err(RejectReason::BadOrderParams));

        let gtd = PlaceOrderRequest {
            time_in_force: TimeInForce::Gtd as i32,
            ..buy()
        };
        assert_eq!(check(gtd.clone()), Err(RejectReason::BadOrderParams));

        let in_the_past = PlaceOrderRequest {
            expire_time: at(-1),
            ..gtd.clone()
        };
        assert_eq!(check(in_the_past), Err(RejectReason::BadOrderParams));

        let in_the_future = PlaceOrderRequest {
            expire_time: at(60),
            ..gtd
        };
        assert_eq!(check(in_the_future), Err(RejectReason::Unsupported));
    }

    #[test]
    fn ioc_and_fok_are_unsupported() {
        for tif in [TimeInForce::Ioc, TimeInForce::Fok] {
            let request = PlaceOrderRequest {
                time_in_force: tif as i32,
                ..buy()
            };
            assert_eq!(check(request), Err(RejectReason::Unsupported));
        }
    }

    #[test]
    fn a_quote_that_does_not_fit_is_a_bad_quantity() {
        let huge = limit_request(proto::Side::Buy, u128::MAX, u128::MAX);
        assert_eq!(check(huge), Err(RejectReason::BadQuantity));
    }
}
