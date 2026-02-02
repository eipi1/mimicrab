# Build stage
FROM rust:1.93-slim-trixie AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    liblua5.4-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/mimicrab
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    liblua5.4-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy the binary from the builder stage
COPY --from=builder /usr/src/mimicrab/target/release/mimicrab .

# Expose the port the app runs on
EXPOSE 3000

# Set the startup command
CMD ["./mimicrab"]
