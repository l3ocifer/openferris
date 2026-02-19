default:
    @just --list

build:
    cargo build --workspace

release:
    cargo build --release -p ferris-core -p ferris-coordinator

check:
    cargo check --workspace

test:
    cargo test --workspace

lint:
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

deny:
    cargo deny check

ci: fmt-check lint test deny

docker-build:
    docker build -t openferris:latest .

docker-up:
    docker compose up --build -d

docker-down:
    docker compose down

docker-logs:
    docker compose logs -f

run-node:
    cargo run -p ferris-core -- start

run-coordinator:
    cargo run -p ferris-coordinator

status:
    cargo run -p ferris-core -- status
