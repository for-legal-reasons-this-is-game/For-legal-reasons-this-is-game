-- false while the backend is recovering from an engine restart: every pod stops placing orders
-- until the consumer has processed the new engine's snapshot. It lives in postgres (and not in the
-- memory of the pod that happens to be the consumer) because any pod can place an order.
ALTER TABLE engine_stream_state
  ADD COLUMN IF NOT EXISTS accepting_orders BOOLEAN NOT NULL DEFAULT TRUE;
