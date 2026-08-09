# ---- Build stage ----
FROM rust:1.90 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
RUN cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim
# ca-certificates so the app can talk to HTTPS URLs if it ever needs to
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/url_shortener /usr/local/bin/url_shortener
EXPOSE 3000
CMD ["url_shortener"]
