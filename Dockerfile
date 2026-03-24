# =========================================================================
# Stage 1: Build the Rust application for a specific target
# =========================================================================
FROM rust:1.88.0-slim AS builder

# Build argument to receive the target platform from the `docker buildx` command
# e.g., linux/amd64 or linux/arm64
ARG TARGETPLATFORM
# buildx sets this per platform; default helps plain `docker build` without --platform
ARG TARGETARCH=amd64

# aws-lc-sys (rustls) needs CMake + a C/C++ toolchain; musl-tools for *-linux-musl link.
# rust-version in Cargo.toml must stay <= image rustc (see FROM above).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    musl-tools \
    && \
    case "${TARGETARCH}" in \
        amd64) rustup target add x86_64-unknown-linux-musl ;; \
        arm64) rustup target add aarch64-unknown-linux-musl ;; \
        *) echo "unsupported TARGETARCH=${TARGETARCH} (need amd64 or arm64)" >&2; exit 1 ;; \
    esac && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Create a dummy project to cache dependencies
RUN cargo init --bin .

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Build only the dependencies for the specified target
# This step is cached as long as Cargo.toml/Cargo.lock don't change.
RUN case "${TARGETARCH}" in \
        amd64) cargo build --release --target x86_64-unknown-linux-musl ;; \
        arm64) cargo build --release --target aarch64-unknown-linux-musl ;; \
        *) echo "unsupported TARGETARCH=${TARGETARCH}" >&2; exit 1 ;; \
    esac

# Now, copy the actual application source code and build it
RUN rm src/*.rs
COPY src ./src
# Embedded at compile time via include_str! in src/i18n.rs (../locales/...)
COPY locales ./locales
RUN case "${TARGETARCH}" in \
        amd64) cargo build --release --target x86_64-unknown-linux-musl ;; \
        arm64) cargo build --release --target aarch64-unknown-linux-musl ;; \
        *) echo "unsupported TARGETARCH=${TARGETARCH}" >&2; exit 1 ;; \
    esac

# Create a symlink to the final build artifact with a predictable name
# This solves the problem of the COPY command in the next stage not being able to use logic.
RUN case "${TARGETARCH}" in \
        amd64) ln -s /usr/src/app/target/x86_64-unknown-linux-musl /usr/src/app/target/final_target ;; \
        arm64) ln -s /usr/src/app/target/aarch64-unknown-linux-musl /usr/src/app/target/final_target ;; \
        *) echo "unsupported TARGETARCH=${TARGETARCH}" >&2; exit 1 ;; \
    esac

# =========================================================================
# Stage 2: Create the final, minimal production image using Alpine
# =========================================================================
FROM alpine:latest

# Install ca-certificates for making HTTPS requests, a common dependency.
RUN apk --no-cache add ca-certificates

WORKDIR /app

# Create necessary directories
RUN mkdir -p /config/embystream

# Copy the configuration file template
COPY src/config/config.toml.template /config/embystream/config.toml

# Copy the compiled binary from the 'builder' stage using the predictable symlink
COPY --from=builder /usr/src/app/target/final_target/release/embystream /app/embystream

# Set the default command to run when the container starts.
CMD ["/app/embystream", "run", "--config", "/config/embystream/config.toml"]
