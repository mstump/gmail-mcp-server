# Build stage
FROM rust:1.91 AS builder

WORKDIR /usr/src/myapp

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy source files
COPY src ./src
COPY templates ./templates

RUN cargo install --path .

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libc6 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/gmail-mcp-server /usr/local/bin/gmail-mcp-server

# Expose ports:
# - 8080: Main HTTP server port (default)
EXPOSE 8080

# Start the executable with --http flag
CMD ["gmail-mcp-server"]

