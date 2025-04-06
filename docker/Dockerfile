FROM rust:slim-bookworm AS builder

WORKDIR /ckeylock
RUN apt update && \
    apt install -y pkg-config libssl-dev openssl

RUN cargo install ckeylock && \
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git

FROM debian:bookworm-slim

WORKDIR /ckeylock

COPY --from=builder /usr/local/cargo/bin/ckeylock /usr/local/bin/ckeylock

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y --no-install-recommends libssl3 pkg-config libssl-dev openssl && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    ldconfig


CMD ["ckeylock"]
