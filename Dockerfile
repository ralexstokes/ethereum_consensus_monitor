FROM rust:1.55 AS builder

RUN USER=root cargo new --bin ethereum-consensus-monitor
WORKDIR /ethereum-consensus-monitor

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src

RUN rm -f ./target/release/deps/ethereum-consensus-monitor*
RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /ethereum-consensus-monitor/target/release/ethereum-consensus-monitor /usr/src/ethereum-consensus-monitor

COPY ./public ./public

RUN apt-get update && apt-get -y upgrade && apt-get install -y --no-install-recommends \
    libssl-dev \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/src/ethereum-consensus-monitor"]
