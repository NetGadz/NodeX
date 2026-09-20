# Multi-stage Dockerfile for Kademlia DHT Node
FROM rust:latest as builder

WORKDIR /usr/src/kademlia-dht

# Install C build tools (gcc) for C FFI compilation
RUN apt-get update && apt-get install -y build-essential gcc libc6-dev && rm -rf /var/lib/apt/lists/*

# Copy project files
COPY Cargo.toml Cargo.lock build.rs ./
COPY core ./core
COPY src ./src

# Build production binary in release mode
RUN cargo build --release

# Final runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install ca-certificates and netcat/curl for health checks
RUN apt-get update && apt-get install -y ca-certificates netcat-openbsd curl && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/src/kademlia-dht/target/release/kademlia-dht /usr/local/bin/kademlia-dht

EXPOSE 8000/udp 8001/udp 8002/udp 8003/udp

ENTRYPOINT ["/usr/local/bin/kademlia-dht"]
CMD ["--port", "8000"]
