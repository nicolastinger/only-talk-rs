# syntax=docker/dockerfile:1

# ---------- build stage ----------
FROM rust:1.92-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config cmake libssl-dev perl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src

RUN cargo build --release --locked

# ---------- runtime stage ----------
FROM debian:bookworm-slim AS runtime

ENV TZ=Asia/Shanghai \
    RUST_LOG=info \
    WORKDIR=/app

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/only_talk_rs /app/only_talk_rs

# HTTP API + QUIC 外网/内网 + NAT UDP 端口
EXPOSE 8443/tcp \
       4433/tcp 4433/udp \
       4434/tcp 4434/udp \
       19562/udp 19563/udp 19564/udp 19565/udp

CMD ["./only_talk_rs"]
