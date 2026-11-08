FROM rust:1.88-alpine AS builder

WORKDIR /app

RUN apk add --no-cache build-base pkgconfig openssl-dev curl-dev

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY static ./static

RUN cargo build --release --locked

FROM alpine:3.22

RUN apk add --no-cache ca-certificates libcurl openssl

WORKDIR /app
COPY --from=builder --chown=65534:65534 /app/target/release/rust-pwa /app/rust-pwa
COPY --from=builder --chown=65534:65534 /app/static /app/static

USER 65534:65534
EXPOSE 3000

ENTRYPOINT ["/app/rust-pwa"]
