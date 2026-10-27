use std::sync::Mutex;

use proto::trading_engine_server::TradingEngine;
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelOrdersRequest, CancelOrdersResponse, EventItem,
    MarketDataItem, ModifyOrderRequest, ModifyOrderResponse, PlaceOrderRequest, PlaceOrderResponse,
    SetMarketRequest, SetMarketResponse, SubscribeEventsRequest, SubscribeMarketDataRequest,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::engine::Engine;

// for now one engine per market but this will change when multi threading is
// impemented
#[derive(Default)]
pub struct EngineService {
    engine: Mutex<Engine>,
}

// the functions from the trait currently have an into_inner() conversion. This is just to silence
// warnings for the meantime so ci/cd doesnt kill it, please make sure to change them (although im
// sure you are going to use into inner anyways)
#[tonic::async_trait]
impl TradingEngine for EngineService {
    async fn set_market(
        &self,
        request: Request<SetMarketRequest>,
    ) -> Result<Response<SetMarketResponse>, Status> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| Status::internal("engine lock poisoned"))?;
        Ok(Response::new(engine.set_market(request.into_inner())))
    }

    async fn place_order(
        &self,
        request: Request<PlaceOrderRequest>,
    ) -> Result<Response<PlaceOrderResponse>, Status> {
        request.into_inner();
        Err(Status::unimplemented("unimplemented"))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        request.into_inner();
        Err(Status::unimplemented("unimplemented"))
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

    type SubscribeEventsStream = ReceiverStream<Result<EventItem, Status>>;

    async fn subscribe_events(
        &self,
        request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
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
