FROM rust:1.83-slim AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY packages ./packages/
COPY sdk ./sdk/
COPY apps/api ./apps/api/

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --bin library-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/library-api /usr/local/bin/
ENV PORT=8080
EXPOSE 8080
CMD ["library-api"]
