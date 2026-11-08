FROM rust:1.89-bookworm AS builder

WORKDIR /app

RUN apt-get update \
	&& apt-get install -y --no-install-recommends pkg-config libssl-dev libcurl4-openssl-dev \
	&& rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

RUN mkdir src \
	&& printf 'fn main() {}\n' > src/main.rs \
	&& rustc --version \
	&& cargo --version \
	&& cargo build --release --locked \
	&& rm -rf src

COPY src ./src
COPY static ./static

RUN cargo clean -p rust-pwa \
	&& cargo build --release --locked \
	&& test -s target/release/rust-pwa

FROM alpine:3.22

RUN apk add --no-cache ca-certificates gcompat libgcc libcurl openssl

WORKDIR /app
COPY --from=builder --chown=65534:65534 /app/target/release/rust-pwa /app/rust-pwa
COPY --from=builder --chown=65534:65534 /app/static /app/static

USER 65534:65534
EXPOSE 3000

ENTRYPOINT ["/app/rust-pwa"]
