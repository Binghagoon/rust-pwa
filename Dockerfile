FROM rust:1.89-bookworm AS builder

WORKDIR /app

RUN apt-get update \
	&& apt-get install -y --no-install-recommends pkg-config libssl-dev libcurl4-openssl-dev \
	&& rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

COPY src ./src
COPY static ./static

RUN --mount=type=cache,id=rust-pwa-cargo-registry,target=/usr/local/cargo/registry \
	--mount=type=cache,id=rust-pwa-cargo-git,target=/usr/local/cargo/git \
	--mount=type=cache,id=rust-pwa-target,target=/app/target \
	rustc --version \
	&& cargo --version \
	&& cargo build --release --locked --bin rust-pwa \
	&& test -s target/release/rust-pwa

FROM alpine:3.22

RUN apk add --no-cache ca-certificates gcompat libgcc libcurl openssl

WORKDIR /app
COPY --from=builder --chown=65534:65534 /app/target/release/rust-pwa /app/rust-pwa
COPY --from=builder --chown=65534:65534 /app/static /app/static

USER 65534:65534
EXPOSE 3000

ENTRYPOINT ["/app/rust-pwa"]
