use proto::trading_engine_client::TradingEngineClient;
use proto::*;
use tonic::transport::Channel;
use tonic::{Status, Streaming};

#[allow(unused)]
#[derive(Clone)]
pub struct EngineClient {
    inner: TradingEngineClient<Channel>,
}

#[allow(unused)]
impl EngineClient {
    pub async fn new(url: &str) -> Result<Self, tonic::transport::Error> {
        let client = TradingEngineClient::connect(url.to_owned()).await.unwrap();
        Ok(EngineClient { inner: client })
    }

    pub async fn set_market(&self, req: SetMarketRequest) -> Result<SetMarketResponse, Status> {
        todo!();
    }

    pub async fn place_order(&self, req: PlaceOrderRequest) -> Result<PlaceOrderResponse, Status> {
        todo!()
    }

    pub async fn cancel_order(
        &self,
        req: CancelOrderRequest,
    ) -> Result<CancelOrderResponse, Status> {
        todo!()
    }

    pub async fn cancel_orders(
        &self,
        req: CancelOrdersRequest,
    ) -> Result<CancelOrdersResponse, Status> {
        todo!()
    }

    pub async fn modify_order(
        &self,
        req: ModifyOrderRequest,
    ) -> Result<ModifyOrderResponse, Status> {
        todo!()
    }

    pub async fn subscribe_events(
        &self,
        req: SubscribeEventsRequest,
    ) -> Result<Streaming<EventItem>, Status> {
        todo!()
    }

    pub async fn subscribe_market_data(
        &self,
        req: SubscribeMarketDataRequest,
    ) -> Result<Streaming<MarketDataItem>, Status> {
        todo!()
    }
}
