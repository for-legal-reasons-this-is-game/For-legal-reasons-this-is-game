CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL
);

CREATE TYPE account_status AS ENUM ('active', 'processing');

CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY,
  account_name TEXT NOT NULL,
  account_ledger_type SMALLINT NOT NULL,
  account_code_type SMALLINT NOT NULL,
  account_user_id UUID REFERENCES users(user_id) NOT NULL,
  account_status account_status NOT NULL DEFAULT 'processing'
);

CREATE TABLE IF NOT EXISTS tb_outbox (
  id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  aggregate_id UUID NOT NULL UNIQUE, -- Id of the account requesting it
  ledger SMALLINT NOT NULL,
  code SMALLINT NOT NULL,
  user_id UUID NOT NULL, --user_data_128 in tb
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  processed_at TIMESTAMPTZ -- Null means still pending 
);
-- this creates a partial index so the relay can be more efficient.
CREATE INDEX IF NOT EXISTS tb_outbox_unprocessed
ON tb_outbox(id) WHERE processed_at IS NULL;
