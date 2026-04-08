FROM rust:1.75 as builder

WORKDIR /app

# Install dependencies for yt-dlp
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp
RUN pip3 install --break-system-packages yt-dlp

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    ffmpeg \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp in runtime
RUN pip3 install --break-system-packages yt-dlp

# Copy built binary
COPY --from=builder /app/target/release/synth /app/synth

# Create index directory
RUN mkdir -p /app/index

# Create non-root user and set ownership
RUN groupadd -r synth && useradd -r -g synth -d /app -s /sbin/nologin synth \
    && chown -R synth:synth /app

USER synth

# Expose port
EXPOSE 8765

# Run the application
CMD ["/app/synth"]
