CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE account_status AS ENUM ('active', 'processing');

CREATE TABLE IF NOT EXISTS ledgers (
  ledger_id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY, -- the TigerBeetle ledger number
  symbol TEXT NOT NULL UNIQUE, -- "USD", "BTC" — the API handle
  name TEXT NOT NULL,
  decimals SMALLINT NOT NULL, -- smallest-unit exponent: USD=2, BTC=8
  enabled BOOLEAN NOT NULL DEFAULT TRUE, --  ledgers can be turned off, not deleted. 
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (decimals >= 0 AND decimals <= 18)
);

INSERT INTO ledgers (symbol, name, decimals) VALUES
  ('USD', 'US Dollar', 2), -- ledger_id = 1
  ('EUR', 'Euro', 2),      -- ledger_id = 2
  ('BTC', 'Bitcoin', 8);   -- ledger_id = 3

CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY,
  account_name TEXT NOT NULL,
  account_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  account_code_type SMALLINT NOT NULL,
  account_user_id UUID REFERENCES users(user_id) NOT NULL,
  account_status account_status NOT NULL DEFAULT 'processing'
);

CREATE TABLE IF NOT EXISTS tb_outbox (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  aggregate_id UUID NOT NULL UNIQUE, -- Id of the account requesting it
  ledger INTEGER NOT NULL,
  code SMALLINT NOT NULL,
  user_id UUID NOT NULL, --user_data_128 in tb
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  processed_at TIMESTAMPTZ -- Null means still pending 
);
-- this creates a partial index so the relay can be more efficient.
CREATE INDEX IF NOT EXISTS tb_outbox_unprocessed
ON tb_outbox(id) WHERE processed_at IS NULL;
