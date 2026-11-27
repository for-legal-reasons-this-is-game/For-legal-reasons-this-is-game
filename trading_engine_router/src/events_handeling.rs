use crate::redis_input::Redis;
use proto::engine_command::Body;
use proto::trading_engine_server::TradingEngine;
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelOrdersRequest, CancelOrdersResponse,
    EngineCommand, MarketDataItem, ModifyOrderRequest, ModifyOrderResponse, PlaceOrderRequest,
    PlaceOrderResponse, SetMarketRequest, SetMarketResponse, SubscribeMarketDataRequest,
};
use std::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// The router only pushes commands to Redis; an engine pod pops and computes them.
/// Responses here only say "queued": the engine's real result does not come back
/// through this path yet.
pub struct EventHandler {
    redis: Mutex<Redis>,
}

impl EventHandler {
    pub fn new(redis: Redis) -> Self {
        Self {
            redis: Mutex::new(redis),
        }
    }

    fn push(&self, market_symbol: &str, body: Body) -> Result<(), Status> {
        // guaranteed by the contract; an empty symbol is a bug, so panic
        assert!(!market_symbol.is_empty(), "market_symbol must be set");
        let command = EngineCommand { body: Some(body) };
        let mut redis = self
            .redis
            .lock()
            .map_err(|_| Status::internal("redis lock poisoned"))?;
        redis
            .push_command(market_symbol, command)
            .map_err(|e| Status::unavailable(format!("redis: {e}")))?;
        Ok(())
    }
}

#[tonic::async_trait]
impl TradingEngine for EventHandler {
    async fn set_market(
        &self,
        request: Request<SetMarketRequest>,
    ) -> Result<Response<SetMarketResponse>, Status> {
        let req = request.into_inner();
        // the contract guarantees `market` is set; a missing one is a bug, so panic
        let symbol = req
            .market
            .as_ref()
            .expect("SetMarketRequest.market must be set")
            .symbol
            .clone();
        self.push(&symbol, Body::SetMarket(req))?;
        // reject_reason unspecified == accepted
        Ok(Response::new(SetMarketResponse::default()))
    }

    async fn place_order(
        &self,
        request: Request<PlaceOrderRequest>,
    ) -> Result<Response<PlaceOrderResponse>, Status> {
        let req = request.into_inner();
        let symbol = req.market_symbol.clone();
        let command_id = req.command_id;
        self.push(&symbol, Body::PlaceOrder(req))?;
        Ok(Response::new(PlaceOrderResponse {
            command_id,
            ..Default::default()
        }))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let req = request.into_inner();
        let symbol = req.market_symbol.clone();
        let command_id = req.command_id;
        self.push(&symbol, Body::CancelOrder(req))?;
        Ok(Response::new(CancelOrderResponse {
            command_id,
            ..Default::default()
        }))
    }

    async fn cancel_orders(
        &self,
        request: Request<CancelOrdersRequest>,
    ) -> Result<Response<CancelOrdersResponse>, Status> {
        request.into_inner();
        Err(Status::unimplemented("unimplemented"))
    }

    async fn modify_order(
        &self,
        request: Request<ModifyOrderRequest>,
    ) -> Result<Response<ModifyOrderResponse>, Status> {
        request.into_inner();
        Err(Status::unimplemented("unimplemented"))
    }

    type SubscribeMarketDataStream = ReceiverStream<Result<MarketDataItem, Status>>;

    async fn subscribe_market_data(
        &self,
        request: Request<SubscribeMarketDataRequest>,
    ) -> Result<Response<Self::SubscribeMarketDataStream>, Status> {
        request.into_inner();
        Err(Status::unimplemented("unimplemented"))
    }
}
