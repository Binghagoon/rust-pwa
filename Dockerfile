FROM rust:1.89-bookworm AS builder

WORKDIR /app

# 시스템 의존성 설치
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 1. 의존성 캐시 극대화를 위해 Cargo 파일만 먼저 복사
COPY Cargo.toml Cargo.lock ./

# 2. 빈 소스 및 스텁(Stub) static 디렉토리를 만들어 의존성만 먼저 빌드
RUN mkdir -p src static \
    && echo "fn main() {}" > src/main.rs \
    && touch static/.keep

# 3. 캐시 마운트를 활용해 의존성 미리 빌드 (소스 변경 시 이 단계는 스킵됨)
RUN --mount=type=cache,id=rust-pwa-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rust-pwa-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rust-pwa-target,target=/app/target \
    cargo build --release --locked --bin rust-pwa

# 4. 실제 소스 코드 및 정적 파일 복사
COPY src ./src
COPY static ./static

# 5. 실제 바이너리 빌드 (소스 코드만 빌드되므로 매우 빠르게 완료됨)
RUN --mount=type=cache,id=rust-pwa-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rust-pwa-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=rust-pwa-target,target=/app/target \
    cargo build --release --locked --bin rust-pwa \
    && test -s target/release/rust-pwa

# --- Runtime Stage ---
FROM alpine:3.22

RUN apk add --no-cache ca-certificates gcompat libgcc libcurl openssl

WORKDIR /app
COPY --from=builder --chown=65534:65534 /app/target/release/rust-pwa /app/rust-pwa
COPY --from=builder --chown=65534:65534 /app/static /app/static

USER 65534:65534
EXPOSE 3000

ENTRYPOINT ["/app/rust-pwa"]