# The musl target produces fully static binaries
FROM rust:1.94-alpine AS builder

# Install musl-dev for static compilation
# Also install build essentials needed by some crates
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build a statically linked binary
RUN cargo build --release --target x86_64-unknown-linux-musl --features aws_s3

# Stage 2: Use scratch for the smallest possible image
# Scratch is an empty image - just the binary and nothing else
FROM scratch

# Copy SSL certificates for HTTPS support
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the statically linked binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/file-server /apollo

EXPOSE 8000

# Scratch images have no shell, so we use the exec form
ENTRYPOINT ["/apollo"]
