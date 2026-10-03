CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE account_status AS ENUM ('active', 'processing');

CREATE TABLE IF NOT EXISTS ledgers (
  ledger_id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY, -- the TigerBeetle ledger number
  symbol TEXT NOT NULL UNIQUE, -- "USD", "BTC"  the API handle
  name TEXT NOT NULL,
  decimals SMALLINT NOT NULL, -- smallest-unit exponent: USD=2, BTC=8
  enabled BOOLEAN NOT NULL DEFAULT TRUE, --  ledgers can be turned off, not deleted. 
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  escrow_account_id UUID NOT NULL REFERENCES accounts(account_id)
  CHECK (decimals >= 0 AND decimals <= 18)
);

INSERT INTO ledgers (symbol, name, decimals) VALUES
  ('USD', 'US Dollar', 2), -- ledger_id = 1
  ('EUR', 'Euro', 2),      -- ledger_id = 2
  ('BTC', 'Bitcoin', 8)    -- ledger_id = 3
ON CONFLICT (symbol) DO NOTHING;

CREATE TABLE IF NOT EXISTS markets (
  market_id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  symbol TEXT NOT NULL UNIQUE, -- "USD-BTC" for example"
  base_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  quote_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  price_decimals SMALLINT NOT NULL,
  fee_account_id UUID NOT NULL REFERENCES accounts(account_id)
  enabled BOOLEAN NOT NULL DEFAULT TRUE, --  market can be turned off, not deleted. 
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (price_decimals >= 0 AND price_decimals <= 18)
);

CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY,
  account_name TEXT NOT NULL,
  account_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  account_code_type SMALLINT NOT NULL,
  account_user_id UUID REFERENCES users(user_id) NOT NULL,
  account_status account_status NOT NULL DEFAULT 'processing',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS orders (
  order_id UUID GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  command_id UUID UNIQUE 
  symbol TEXT NOT NULL UNIQUE, -- "USD-BTC" for example"
  base_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  quote_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  price_decimals SMALLINT NOT NULL,
  fee_account_id UUID NOT NULL REFERENCES accounts(account_id)
  enabled BOOLEAN NOT NULL DEFAULT TRUE, --  ledgers can be turned off, not deleted. 
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (price_decimals >= 0 AND price_decimals <= 18)
);

create table if not exists tb_outbox (
  id bigint generated always as identity primary key,
  aggregate_id uuid not null unique, -- id of the account requesting it
  ledger integer not null,
  code smallint not null,
  user_id uuid not null, --user_data_128 in tb
  created_at timestamptz not null default now(),
  processed_at timestamptz -- null means still pending 
);
-- this creates a partial index so the relay can be more efficient.
CREATE INDEX IF NOT EXISTS tb_outbox_unprocessed
ON tb_outbox(id) WHERE processed_at IS NULL;
