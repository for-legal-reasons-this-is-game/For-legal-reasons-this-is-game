CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  account_name TEXT
  account_ledger_type SMALLINT NOT NULL
  account_code_type SMALLINT NOT NULL
  FOREIGN KEY (user_id) REFERENCES
  users(user_id);
);

CREATE TABLE IF NOT EXISTS ledger (
  ledger_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  ledger_type TEXT NOT NULL
  FOREIGN KEY (account_id) REFERENCES
  accounts(account_id);
);

