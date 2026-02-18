-- Network storage: tracks which node stores which file
CREATE TABLE IF NOT EXISTS network_objects (
    object_id       TEXT PRIMARY KEY,
    owner_agent     TEXT NOT NULL REFERENCES agents(agent_id),
    storage_agent   TEXT NOT NULL REFERENCES agents(agent_id),
    name            TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_network_objects_owner ON network_objects(owner_agent);
CREATE INDEX IF NOT EXISTS idx_network_objects_storage ON network_objects(storage_agent);
CREATE INDEX IF NOT EXISTS idx_network_objects_hash ON network_objects(content_hash);
