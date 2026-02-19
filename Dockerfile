FROM ubuntu:24.04 AS builder
WORKDIR /app

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl build-essential pkg-config libssl-dev g++ cmake ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
RUN cargo fetch

COPY migrations/ migrations/
COPY config/ config/

RUN cargo build --release -p ferris-core -p ferris-coordinator \
    && strip target/release/ferris target/release/ferris-coordinator

FROM ubuntu:24.04 AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3t64 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r ferris && useradd -r -g ferris -m ferris \
    && mkdir -p /var/ferris && chown ferris:ferris /var/ferris

COPY --from=builder /app/target/release/ferris /usr/local/bin/ferris
COPY --from=builder /app/target/release/ferris-coordinator /usr/local/bin/ferris-coordinator
COPY config/default.toml /etc/ferris/default.toml

USER ferris
WORKDIR /home/ferris

ENV RUST_LOG=info
ENV FERRIS_DATA_DIR=/home/ferris/.ferris

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8421/health || exit 1

EXPOSE 8420 8421

ENTRYPOINT ["ferris-coordinator"]
