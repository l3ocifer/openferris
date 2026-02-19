FROM rust:bookworm AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
RUN cargo fetch

FROM rust:bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev g++ cmake \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/ .
COPY --from=planner /usr/local/cargo/registry /usr/local/cargo/registry

COPY migrations/ migrations/
COPY config/ config/

RUN cargo build --release -p ferris-core -p ferris-coordinator \
    && strip target/release/ferris target/release/ferris-coordinator

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r ferris && useradd -r -g ferris -m ferris \
    && mkdir -p /var/ferris && chown ferris:ferris /var/ferris

COPY --from=builder /app/target/release/ferris /usr/local/bin/ferris
COPY --from=builder /app/target/release/ferris-coordinator /usr/local/bin/ferris-coordinator
COPY config/default.toml /etc/ferris/default.toml
COPY migrations/ /etc/ferris/migrations/

USER ferris
WORKDIR /home/ferris

ENV RUST_LOG=info
ENV FERRIS_DATA_DIR=/home/ferris/.ferris

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8421/health || exit 1

EXPOSE 8420 8421

ENTRYPOINT ["ferris-coordinator"]
