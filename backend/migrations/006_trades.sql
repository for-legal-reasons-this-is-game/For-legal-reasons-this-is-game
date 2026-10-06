CREATE TABLE IF NOT EXISTS trades (
  trade_id UUID PRIMARY KEY,
  market_id INTEGER REFERENCES markets(market_id) NOT NULL,
  buyer_order_id UUID REFERENCES orders(order_id) NOT NULL,
  seller_order_id UUID REFERENCES orders(order_id) NOT NULL,
  CONSTRAINT trades_distinct_orders CHECK (buyer_order_id <> seller_order_id),
  price NUMERIC(39, 0) NOT NULL CHECK (price > 0),
  base_quantity NUMERIC(39, 0) NOT NULL CHECK (base_quantity > 0),
  quote_quantity NUMERIC(39, 0) NOT NULL CHECK (quote_quantity > 0),
  buyer_fee NUMERIC(39, 0) NOT NULL,
  seller_fee NUMERIC(39, 0) NOT NULL,
  taker_side SMALLINT NOT NULL CHECK (taker_side IN (1, 2)),
  engine_epoch BIGINT NOT NULL,
  engine_sequence BIGINT NOT NULL,
  engine_time TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- buy orders lopokup
CREATE INDEX IF NOT EXISTS trades_buyer_order ON trades(buyer_order_id);
-- sell orders lookup
CREATE INDEX IF NOT EXISTS trades_seller_order ON trades(seller_order_id);
-- latest trades, newest first so the frontend can quickly check
CREATE INDEX IF NOT EXISTS trades_market_time ON trades(market_id, engine_time DESC);
