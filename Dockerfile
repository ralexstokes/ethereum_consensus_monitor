FROM rust:1.55 AS builder

RUN USER=root cargo new --bin ethereum_consensus_monitor
WORKDIR /ethereum_consensus_monitor

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm -f ./target/release/deps/ethereum_consensus_monitor*
RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /ethereum_consensus_monitor/target/release/ethereum_consensus_monitor /usr/src/ethereum_consensus_monitor

COPY ./public ./public

RUN apt-get update && apt-get -y upgrade && apt-get install -y --no-install-recommends \
    libssl-dev \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/src/ethereum_consensus_monitor"]
