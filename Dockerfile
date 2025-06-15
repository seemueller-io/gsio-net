# Multistage Dockerfile for gsio-node

# Build stage
FROM rust:slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty project
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/gsio-node/Cargo.toml ./crates/gsio-node/
COPY crates/gsio-relay/Cargo.toml ./crates/gsio-relay/
COPY crates/gsio-client/Cargo.toml ./crates/gsio-client/
COPY crates/gsio-wallet/Cargo.toml ./crates/gsio-wallet/

# Create dummy source files to build dependencies
RUN mkdir -p crates/gsio-node/src && \
    echo 'fn main() { println!("Dummy!"); }' > crates/gsio-node/src/main.rs && \
    mkdir -p crates/gsio-relay/src && \
    echo 'fn main() { println!("Dummy!"); }' > crates/gsio-relay/src/lib.rs && \
    mkdir -p crates/gsio-client/src && \
    echo 'fn main() { println!("Dummy!"); }' > crates/gsio-client/src/main.rs && \
    mkdir -p crates/gsio-wallet/src && \
    echo 'pub fn dummy() {}' > crates/gsio-wallet/src/lib.rs

# Create dummy source files to build dependencies


# Build dependencies - this will be cached if dependencies don't change
RUN cargo build --release --bin gsio-node

# Remove the dummy source files
RUN rm -rf crates/*/src

# Copy the actual source code
COPY crates ./crates

# Build the application
RUN cargo build --release --bin gsio-node

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user to run the application
RUN useradd -m appuser
USER appuser
WORKDIR /home/appuser

# Copy the binary from the builder stage
COPY --from=builder --chown=appuser:appuser /app/target/release/gsio-node .

# Expose the port the app runs on
EXPOSE 3000

# Command to run the application
CMD ["./gsio-node"]