use proto::trading_engine_client::TradingEngineClient;
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelOrdersRequest, CancelOrdersResponse, EventItem,
    MarketDataItem, ModifyOrderRequest, ModifyOrderResponse, PlaceOrderRequest, PlaceOrderResponse,
    SetMarketRequest, SetMarketResponse, SubscribeEventsRequest, SubscribeMarketDataRequest,
};

use tonic::transport::{Channel, Endpoint};
use tonic::{Status, Streaming};

#[allow(unused)]
#[derive(Clone)]
pub struct EngineClient {
    inner: TradingEngineClient<Channel>,
}

#[allow(unused)]
impl EngineClient {
    pub fn new(url: &str) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(url.to_owned())?.connect_lazy();
        Ok(EngineClient {
            inner: TradingEngineClient::new(channel),
        })
    }

    pub async fn set_market(&self, req: SetMarketRequest) -> Result<SetMarketResponse, Status> {
        let mut client = self.inner.clone();
        let response = client.set_market(req).await?;
        Ok(response.into_inner())
    }

    pub async fn place_order(&self, req: PlaceOrderRequest) -> Result<PlaceOrderResponse, Status> {
        let mut client = self.inner.clone();
        let response = client.place_order(req).await?;
        Ok(response.into_inner())
    }

    pub async fn cancel_order(
        &self,
        req: CancelOrderRequest,
    ) -> Result<CancelOrderResponse, Status> {
        let mut client = self.inner.clone();
        let response = client.cancel_order(req).await?;
        Ok(response.into_inner())
    }

    pub async fn cancel_orders(
        &self,
        req: CancelOrdersRequest,
    ) -> Result<CancelOrdersResponse, Status> {
        let mut client = self.inner.clone();
        let response = client.cancel_orders(req).await?;
        Ok(response.into_inner())
    }

    pub async fn modify_order(
        &self,
        req: ModifyOrderRequest,
    ) -> Result<ModifyOrderResponse, Status> {
        let mut client = self.inner.clone();
        let response = client.modify_order(req).await?;
        Ok(response.into_inner())
    }

    pub async fn subscribe_events(
        &self,
        req: SubscribeEventsRequest,
    ) -> Result<Streaming<EventItem>, Status> {
        let mut client = self.inner.clone();
        let response = client.subscribe_events(req).await?;
        Ok(response.into_inner())
    }

    pub async fn subscribe_market_data(
        &self,
        req: SubscribeMarketDataRequest,
    ) -> Result<Streaming<MarketDataItem>, Status> {
        let mut client = self.inner.clone();
        let response = client.subscribe_market_data(req).await?;
        Ok(response.into_inner())
    }
}
