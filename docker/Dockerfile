FROM rust:latest AS builder

WORKDIR /ckeylock

RUN cargo install ckeylock && \
    rm -rf /usr/local/cargo/registry /usr/local/cargo/git

FROM debian:bullseye-slim

WORKDIR /ckeylock

COPY --from=builder /usr/local/cargo/bin/ckeylock /usr/local/bin/ckeylock


CMD ["ckeylock"]
