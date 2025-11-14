# Build stage
FROM rust:1.91-trixie AS builder

WORKDIR /usr/src/app

# Create a dummy project to cache dependencies
RUN cargo init --bin .

# Copy over the dependency definitions
COPY Cargo.toml Cargo.lock ./

# Build the dependencies with cache mounts. This will be cached as a layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=target \
    touch src/main.rs && \
    cargo build --release

# Now copy over the actual source code
COPY src ./src
COPY templates ./templates

# Build the application, which should be fast because dependencies are cached
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=target \
    cargo install --path .

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

# Start the executable
CMD ["gmail-mcp-server", "http"]

