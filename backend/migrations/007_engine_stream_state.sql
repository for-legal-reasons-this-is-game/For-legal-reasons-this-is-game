CREATE TABLE IF NOT EXISTS engine_stream_state (
  id SMALLINT PRIMARY KEY CHECK (id = 1),
  engine_epoch BIGINT NOT NULL DEFAULT 0,
  last_sequence BIGINT NOT NULL DEFAULT 0,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO engine_stream_state (id) VALUES (1)
ON CONFLICT (id) DO NOTHING;
