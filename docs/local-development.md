# Local Development Quickstart

This guide gets a runnable local OpenFerris dev server up in minutes.

## Prerequisites

- Rust toolchain (see `rust-toolchain.toml`)
- Optional: [Ollama](https://ollama.com) for local inference

## 1) Quick start (recommended)

```bash
cargo run -p ferris-core -- start --host 127.0.0.1 --port 8420
```

This auto-initializes the node (data dir, config, DB, Ed25519 identity), detects resources, and starts the HTTP server with encryption, semantic search, and task execution.

You should see:

```text
Initialized OpenFerris node
  agent_id: 019c6db1-...

Detected resources:
  cpu: 10 cores, ram: 65536 MB, storage: 524288 MB
  ollama: not detected

Contributing 50% of resources:
  cpu: 5 cores, ram: 32768 MB, storage: 262144 MB

HTTP server:  http://127.0.0.1:8420
Encryption:   AES-256-GCM (at rest)

Ready. Earning credits from contributed resources.
```

## 2) Alternative: serve only (no network)

```bash
cargo run -p ferris-core -- init
cargo run -p ferris-core -- serve --transport http --host 127.0.0.1 --port 8420
```

## 3) Health check

```bash
curl -s http://127.0.0.1:8420/health | jq
```

## 4) Memory APIs (with semantic search)

```bash
# Store a memory
curl -s -X POST http://127.0.0.1:8420/api/v1/memory/remember \
  -H "content-type: application/json" \
  -d '{"key":"favourite_color","value":"azure blue"}'

# Semantic recall — finds "azure blue" even when querying "color"
curl -s -X POST http://127.0.0.1:8420/api/v1/memory/recall \
  -H "content-type: application/json" \
  -d '{"query":"what shade do I like","limit":10}'

# Delete a memory
curl -i -X DELETE http://127.0.0.1:8420/api/v1/memory/favourite_color
```

Memory values are encrypted at rest (AES-256-GCM) using a key derived from the node's Ed25519 identity. Semantic search uses the `fastembed` crate with AllMiniLM-L6-V2 embeddings (384-dim, ~30MB model auto-downloaded on first recall).

## 5) Storage APIs

```bash
DATA=$(printf "hello ferris" | base64)

curl -s -X POST http://127.0.0.1:8420/api/v1/storage/store \
  -H "content-type: application/json" \
  -d "{\"name\":\"hello.txt\",\"data_base64\":\"$DATA\"}"

curl -s http://127.0.0.1:8420/api/v1/storage

# Replace file-1 with returned file_id
curl -s http://127.0.0.1:8420/api/v1/storage/file-1
```

File contents are encrypted at rest before writing to disk. Content-addressed deduplication (blake3) operates on plaintext hashes.

## 6) Task APIs (with cron execution)

```bash
# Schedule a task (cron expression + JSON action)
curl -s -X POST http://127.0.0.1:8420/api/v1/tasks \
  -H "content-type: application/json" \
  -d '{"schedule":"*/5 * * * *","action":"{\"type\":\"log\",\"message\":\"heartbeat\"}"}'

curl -s http://127.0.0.1:8420/api/v1/tasks

curl -i -X DELETE http://127.0.0.1:8420/api/v1/tasks/task-1
```

Supported action types:
- `{"type":"log","message":"..."}` — writes to tracing output
- `{"type":"http","url":"https://...","body":"..."}` — sends POST request
- `{"type":"webhook","url":"https://...","body":"..."}` — alias for http

The task executor polls every 60 seconds and evaluates cron expressions to determine which tasks are due.

## 7) Inference API (requires Ollama)

```bash
curl -s -X POST http://127.0.0.1:8420/v1/chat/completions \
  -H "content-type: application/json" \
  -d '{"model":"llama3","messages":[{"role":"user","content":"Hello!"}]}'

curl -s http://127.0.0.1:8420/v1/models
```

## 9) Network Storage APIs

These endpoints require a running coordinator and at least one other registered node.

```bash
# Store a file on the network (coordinator routes to a storage node)
curl -s -X POST https://api.openferris.com/api/v1/network/store \
  -H "X-Agent-Id: $AGENT_ID" -H "X-Signature: $SIG" \
  -H "content-type: application/json" \
  -d '{"name":"report.pdf","data_base64":"..."}'

# List your network files
curl -s https://api.openferris.com/api/v1/network/files \
  -H "X-Agent-Id: $AGENT_ID"

# Retrieve a file from the network
curl -s https://api.openferris.com/api/v1/network/files/$OBJECT_ID \
  -H "X-Agent-Id: $AGENT_ID"
```

Storage is settled at 1mc/KB with a 15% platform fee. The coordinator tracks file locations in the `network_objects` table and proxies retrieval requests to the storing node.

## 8) Running tests

```bash
cargo test --workspace
```

## Current Features

| Feature | Implementation |
|---------|---------------|
| Memory | SQLite with upsert, semantic vector search (fastembed), encrypted at rest |
| Storage | Content-addressed (blake3), encrypted at rest, quota enforcement |
| Tasks | Cron scheduling (croner), background execution loop, run history |
| Inference | Ollama proxy with OpenAI-compatible API |
| Identity | Ed25519 keypair, deterministic agent ID |
| Encryption | AES-256-GCM via HKDF-derived key from Ed25519 secret |
| Network | Coordinator registration, heartbeat, credit economy, inference routing, network storage |

Canonical scope and next milestones are in:
- `docs/PRD.md`
- `docs/spec-v1.md`
- `docs/agent-interoperability.md`
