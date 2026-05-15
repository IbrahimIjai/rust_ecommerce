CREATE TABLE webhook_events (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type   TEXT NOT NULL,
    reference    TEXT,
    payload      JSONB NOT NULL,
    processed_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (reference, event_type)
);

CREATE INDEX IF NOT EXISTS idx_webhook_events_reference ON webhook_events(reference);
