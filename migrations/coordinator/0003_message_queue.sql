-- Agent-to-agent message queue with 24-hour TTL
CREATE TABLE IF NOT EXISTS message_queue (
    message_id      TEXT PRIMARY KEY,
    from_agent      TEXT NOT NULL REFERENCES agents(agent_id),
    to_agent        TEXT NOT NULL REFERENCES agents(agent_id),
    payload         TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    expires_at      INTEGER NOT NULL,
    delivered_at    INTEGER
);

CREATE INDEX IF NOT EXISTS idx_messages_to ON message_queue(to_agent);
CREATE INDEX IF NOT EXISTS idx_messages_expires ON message_queue(expires_at);
