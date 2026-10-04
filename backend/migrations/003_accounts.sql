CREATE TYPE account_status AS ENUM ('active', 'processing');

CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY,
  account_name TEXT NOT NULL,
  account_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  account_code_type SMALLINT NOT NULL CHECK (account_code_type IN (1, 2, 3)),
  account_user_id UUID REFERENCES users(user_id) NOT NULL,
  account_status account_status NOT NULL DEFAULT 'processing',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (account_id, account_ledger_id)
);

CREATE INDEX IF NOT EXISTS accounts_user ON accounts(account_user_id);
CREATE UNIQUE INDEX IF NOT EXISTS accounts_one_system_account_per_ledger
ON accounts(account_ledger_id, account_code_type) WHERE account_code_type IN (2, 3);

create table if not exists tb_outbox(
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
