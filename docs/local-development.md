# Local Development Quickstart

This guide gets a runnable local OpenFerris dev server up in minutes.

## 1) Start the server

```bash
cargo run -p ferris-core -- serve --host 127.0.0.1 --port 8420
```

You should see:

```text
ferris local dev server listening on http://127.0.0.1:8420
```

## 2) Health check

```bash
curl -s http://127.0.0.1:8420/health
```

## 3) Memory APIs

```bash
curl -s -X POST http://127.0.0.1:8420/api/v1/memory/remember \
  -H "content-type: application/json" \
  -d '{"key":"project","value":"openferris"}'

curl -s -X POST http://127.0.0.1:8420/api/v1/memory/recall \
  -H "content-type: application/json" \
  -d '{"query":"open","limit":10}'

curl -i -X DELETE http://127.0.0.1:8420/api/v1/memory/project
```

## 4) Storage APIs

```bash
DATA=$(printf "hello ferris" | base64)

curl -s -X POST http://127.0.0.1:8420/api/v1/storage/store \
  -H "content-type: application/json" \
  -d "{\"name\":\"hello.txt\",\"data_base64\":\"$DATA\"}"

curl -s http://127.0.0.1:8420/api/v1/storage

# Replace file-1 with returned file_id
curl -s http://127.0.0.1:8420/api/v1/storage/file-1
```

## 5) Task APIs

```bash
curl -s -X POST http://127.0.0.1:8420/api/v1/tasks \
  -H "content-type: application/json" \
  -d '{"schedule":"*/5 * * * *","action":"echo ping"}'

curl -s http://127.0.0.1:8420/api/v1/tasks

curl -i -X DELETE http://127.0.0.1:8420/api/v1/tasks/task-1
```

## Current Scope

This local server is a development bootstrap:
1. In-memory state only (resets on restart).
2. No coordinator/network calls yet.
3. No auth/signatures yet.

Canonical scope and next milestones are in:
- `docs/PRD.md`
- `docs/spec-v1.md`
- `docs/agent-interoperability.md`
