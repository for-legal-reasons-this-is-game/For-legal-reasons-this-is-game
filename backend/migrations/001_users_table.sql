CREATE TABLE IF NOT EXISTS users (
  user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_name TEXT NOT NULL UNIQUE
);
-- CREATE TABLE IF NOT EXISTS accounts (
--   account_id SERIAL PRIMARY KEY,
--   account_name TEXT NOT NULL UNIQUE
--   FOREIGN KEY (user_id) REFERENCES
--   users(user_id);
-- );

