CREATE TABLE IF NOT EXISTS ledgers (
  ledger_id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY, -- the TigerBeetle ledger number
  symbol TEXT NOT NULL UNIQUE, -- "USD", "BTC"  the API handle
  name TEXT NOT NULL,
  decimals SMALLINT NOT NULL, -- smallest-unit exponent: USD=2, BTC=8
  enabled BOOLEAN NOT NULL DEFAULT TRUE, --  ledgers can be turned off, not deleted. 
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CHECK (decimals >= 0 AND decimals <= 18)
);

INSERT INTO ledgers (symbol, name, decimals) VALUES
  ('USD', 'US Dollar', 2), -- ledger_id = 1
  ('EUR', 'Euro', 2),      -- ledger_id = 2
  ('BTC', 'Bitcoin', 8)    -- ledger_id = 3
ON CONFLICT (symbol) DO NOTHING;
