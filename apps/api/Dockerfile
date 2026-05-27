FROM rust:bookworm AS builder

WORKDIR /workspace

# Build the API image from the repository root so workspace crates are available.
RUN rustup default nightly \
  && apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

COPY . .

ENV SQLX_OFFLINE=true
RUN cargo build --release --package library-api --bin library-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/library-api /usr/local/bin/library-api

ENV ENVIRONMENT=production
ENV PORT=8080
EXPOSE 8080

CMD ["/usr/local/bin/library-api"]
