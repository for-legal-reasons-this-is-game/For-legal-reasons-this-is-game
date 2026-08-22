CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL
  user_birthdate DATE NOT NULL
);
CREATE TABLE IF NOT EXISTS accounts (
  account_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  account_name TEXT
  FOREIGN KEY (user_id) REFERENCES
  users(user_id);
);

CREATE TABLE IF NOT EXISTS instruments (
  account_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  account_name TEXT
  FOREIGN KEY (user_id) REFERENCES
  users(user_id);
);

