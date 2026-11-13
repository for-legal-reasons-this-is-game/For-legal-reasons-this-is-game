use proto::trading_engine_server::TradingEngine;
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelOrdersRequest, CancelOrdersResponse,
    ConfigRejectReason, EventItem, MarketDataItem, ModifyOrderRequest, ModifyOrderResponse,
    PlaceOrderRequest, PlaceOrderResponse, RejectReason, SetMarketRequest, SetMarketResponse,
    SubscribeEventsRequest, SubscribeMarketDataRequest,
};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::engine::Engine;
use crate::store::RedisStore;

/// The engine's gRPC service.
///
/// Commands run one at a time. With a [`RedisStore`], each command that changes a market is
/// saved before its answer is sent, so a restarted engine never forgets something the backend
/// was told. Without one, the state lives only in memory.
#[derive(Default)]
pub struct EngineService {
    state: Mutex<State>,
}

/// The engine and where its state is saved. One lock covers both, so a save always sees the
/// state its command left behind.
#[derive(Default)]
struct State {
    engine: Engine,
    store: Option<RedisStore>,
}

impl EngineService {
    /// A service that saves its state to `store`, starting from `engine`, which is usually
    /// what [`RedisStore::load`] returned.
    pub fn with_store(engine: Engine, store: RedisStore) -> Self {
        Self {
            state: Mutex::new(State {
                engine,
                store: Some(store),
            }),
        }
    }
}

impl State {
    /// Saves the market a command just changed, before the command's answer is sent.
    ///
    /// As agreed at the weekly meeting, an unexpected failure panics: if Redis can't take the
    /// save, the engine stops. Answering a command whose change isn't saved would let a
    /// restart forget something the backend was told. The restarted engine loads the last
    /// saved state, and the backend retries the command that got no answer.
    async fn save(&mut self, symbol: &str) {
        let Some(store) = &mut self.store else {
            return;
        };
        if let Err(error) = store.save_market(&self.engine, symbol).await {
            panic!("ENGINE: couldn't save market {symbol} to redis, stopping: {error}");
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
        let request = request.into_inner();
        let symbol = request.market.as_ref().map(|market| market.symbol.clone());
        let mut state = self.state.lock().await;
        let response = state.engine.set_market(request);
        // a rejected command changed nothing, so there is nothing to save
        let accepted = response.reject_reason() == ConfigRejectReason::Unspecified;
        if accepted && let Some(symbol) = symbol {
            state.save(&symbol).await;
        }
        Ok(Response::new(response))
    }

    async fn place_order(
        &self,
        request: Request<PlaceOrderRequest>,
    ) -> Result<Response<PlaceOrderResponse>, Status> {
        let request = request.into_inner();
        let symbol = request.market_symbol.clone();
        let mut state = self.state.lock().await;
        let response = state.engine.place_order(request);
        if response.reject_reason() == RejectReason::Unspecified {
            state.save(&symbol).await;
        }
        Ok(Response::new(response))
    }

    async fn cancel_order(
        &self,
        request: Request<CancelOrderRequest>,
    ) -> Result<Response<CancelOrderResponse>, Status> {
        let request = request.into_inner();
        let symbol = request.market_symbol.clone();
        let mut state = self.state.lock().await;
        let response = state.engine.cancel_order(request);
        if response.reject_reason() == RejectReason::Unspecified {
            state.save(&symbol).await;
        }
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
