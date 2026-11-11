use std::sync::Arc;

use proto::place_order_request::Amount;
use proto::trading_engine_server::TradingEngine;
use proto::{
    ConfigRejectReason, Id, Market, OrderStatus, OrderType, PlaceOrderRequest, RejectReason,
    SetMarketRequest, Side, TimeInForce, Uint128,
};
use tonic::Request;
use tradingengine::service::EngineService;

// these call the grpc handler itself, the same function tonic calls for a real
// request, so they cover the lock and the request/response wrapping on top of
// Engine. no network is involved, tests/engine.rs covers the engine logic

fn market(symbol: &str) -> Market {
    Market {
        symbol: symbol.to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals: 2,
        fee_account: Some(Id::from(1u128)),
    }
}

async fn set(service: &EngineService, market: Option<Market>) -> ConfigRejectReason {
    service
        .set_market(Request::new(SetMarketRequest { market }))
        .await
        .expect("set_market answers rejections in the response, never as a grpc error")
        .into_inner()
        .reject_reason()
}

#[tokio::test]
async fn set_market_is_accepted_through_the_handler() {
    let service = EngineService::default();

    assert_eq!(
        set(&service, Some(market("BTC-USD"))).await,
        ConfigRejectReason::Unspecified
    );
}

#[tokio::test]
async fn a_rejection_is_a_response_not_a_grpc_error() {
    // the contract puts rejections in reject_reason, the call itself succeeds
    let service = EngineService::default();

    assert_eq!(set(&service, None).await, ConfigRejectReason::BadDefinition);
}

#[tokio::test]
async fn calls_from_many_tasks_at_once_all_get_through_the_lock() {
    // tonic runs every request in its own task, this checks they share the one
    // engine behind the mutex without deadlocking or losing a call
    let service = Arc::new(EngineService::default());

    let tasks: Vec<_> = (0..20)
        .map(|n| {
            let service = Arc::clone(&service);
            tokio::spawn(async move { set(&service, Some(market(&format!("COIN{n}-USD")))).await })
        })
        .collect();

    for task in tasks {
        assert_eq!(
            task.await.expect("task finished"),
            ConfigRejectReason::Unspecified
        );
    }
}

#[tokio::test]
async fn place_order_goes_through_the_handler_to_the_same_engine() {
    // set_market and place_order are separate calls, they must reach one engine
    let service = EngineService::default();
    set(&service, Some(market("BTC-USD"))).await;

    let mut request = PlaceOrderRequest {
        command_id: Some(Id::from(1u128)),
        order_id: Some(Id::from(2u128)),
        market_symbol: "BTC-USD".to_string(),
        account_holder: Some(Id::from(3u128)),
        base_account: Some(Id::from(4u128)),
        quote_account: Some(Id::from(5u128)),
        amount: Some(Amount::BaseQuantity(Uint128::from(50_000_000u128))),
        limit_price: Some(Uint128::from(2_900_000u128)),
        ..PlaceOrderRequest::default()
    };
    request.set_side(Side::Buy);
    request.set_type(OrderType::Limit);
    request.set_time_in_force(TimeInForce::Gtc);

    let response = service
        .place_order(Request::new(request))
        .await
        .expect("place_order answers rejections in the response, never as a grpc error")
        .into_inner();

    assert_eq!(response.reject_reason(), RejectReason::Unspecified);
    assert_eq!(response.status(), OrderStatus::New);
}
