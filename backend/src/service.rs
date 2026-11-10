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

    /// Same as [`new`](Self::new), but with HTTP/2 keepalive pings.
    ///
    /// That way a connection that died silently (a proxy dropped it, or the router vanished) is
    /// noticed, instead of leaving a stream waiting forever.
    pub fn with_keepalive(
        url: &str,
        interval: std::time::Duration,
        timeout: std::time::Duration,
    ) -> Result<Self, tonic::transport::Error> {
        let channel = Endpoint::from_shared(url.to_owned())?
            .http2_keep_alive_interval(interval)
            .keep_alive_timeout(timeout)
            .keep_alive_while_idle(true)
            .connect_lazy();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn with_keepalive_accepts_a_url_and_rejects_garbage() {
        let ok = EngineClient::with_keepalive(
            "http://127.0.0.1:50051",
            Duration::from_secs(20),
            Duration::from_secs(10),
        );
        assert!(ok.is_ok());
        let bad = EngineClient::with_keepalive("not a url", Duration::from_secs(20), Duration::from_secs(10));
        assert!(bad.is_err());
    }
}
