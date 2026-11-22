use proto::trading_engine_server::TradingEngine;
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelOrdersRequest, CancelOrdersResponse, EventItem,
    MarketDataItem, ModifyOrderRequest, ModifyOrderResponse, PlaceOrderRequest, PlaceOrderResponse,
    SetMarketRequest, SetMarketResponse, SubscribeEventsRequest, SubscribeMarketDataRequest,
};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::engine::Engine;
use crate::redis_store::RedisStore;

#[derive(Default)]
pub struct State {
    engine: Engine,
    store: Option<RedisStore>,
}

impl State {
    async fn save(&mut self) {
        let Some(store) = &mut self.store else {
            return;
        };
        let state_bytes = self
            .engine
            .save()
            .to_bytes()
            .expect("error converting `EngineState` to bytes");
        match store.save(&state_bytes).await {
            Ok(_) => {}
            Err(error) => panic!("error saving in `RedisStore`: {error}"),
        };
    }
}

/// The engine's gRPC service.
#[derive(Default)]
pub struct EngineService {
    // This is now a tokio Mutex
    state: Mutex<State>,
}

impl EngineService {
    pub fn new(engine: Engine, store: Option<RedisStore>) -> Self {
        EngineService {
            state: Mutex::new(State { engine, store }),
        }
    }
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
        // as talked over weekly meet up unspecified errors should panic
        // expect panics when err comes from lock
        let mut state = self.state.lock().await;
        let response = state.engine.set_market(request.into_inner());
        state.save().await;
        Ok(Response::new(response))
    }

    async fn place_order(
        &self,
        request: Request<PlaceOrderRequest>,
    ) -> Result<Response<PlaceOrderResponse>, Status> {
        let mut state = self.state.lock().await;
        let response = state.engine.place_order(request.into_inner());
        state.save().await;
        Ok(Response::new(response))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let mut state = self.state.lock().await;
        let response = state.engine.cancel_order(request.into_inner());
        state.save().await;
        Ok(Response::new(response))
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
