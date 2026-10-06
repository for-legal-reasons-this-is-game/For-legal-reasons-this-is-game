CREATE TYPE market_status AS ENUM ('trading', 'cancel_only', 'halted');

CREATE TABLE IF NOT EXISTS markets (
  market_id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  symbol TEXT NOT NULL UNIQUE, -- "BTC-USD" for example
  base_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  quote_ledger_id INTEGER NOT NULL REFERENCES ledgers(ledger_id),
  price_decimals SMALLINT NOT NULL,
  fee_account_id UUID NOT NULL,
  status market_status NOT NULL DEFAULT 'halted',
  min_base_quantity NUMERIC(39, 0) NOT NULL CHECK (min_base_quantity > 0),
  min_quote_quantity NUMERIC(39, 0) NOT NULL CHECK (min_quote_quantity > 0),
  quantity_step NUMERIC(39, 0) NOT NULL DEFAULT 1 CHECK (quantity_step > 0),
  price_step NUMERIC(39, 0) NOT NULL DEFAULT 1 CHECK (price_step > 0),
  maker_fee_bps INTEGER NOT NULL,
  taker_fee_bps INTEGER NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (price_decimals >= 0 AND price_decimals <= 18),
  CHECK (base_ledger_id <> quote_ledger_id),
  UNIQUE (base_ledger_id, quote_ledger_id),
  FOREIGN KEY (fee_account_id, quote_ledger_id) REFERENCES accounts(account_id, account_ledger_id)
);
