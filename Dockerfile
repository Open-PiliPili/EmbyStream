# =========================================================================
# Stage 1: Build the Rust application for a specific target
# =========================================================================
FROM rust:1.88.0-slim AS builder

# BuildKit sets TARGETPLATFORM per --platform (e.g. linux/amd64, linux/arm64).
# Do NOT default TARGETARCH to amd64: multi-platform + cache can otherwise
# compile the wrong triple (e.g. x86_64-musl under an arm64 target), breaking aws-lc-sys (-m64).
ARG TARGETPLATFORM
ARG TARGETARCH

# Resolve musl triple from TARGETPLATFORM (single source of truth for cross/QEMU builds).
RUN set -eu; \
    case "${TARGETPLATFORM:-linux/amd64}" in \
        linux/amd64) echo amd64 > /musl-arch; echo x86_64-unknown-linux-musl > /rust-target ;; \
        linux/arm64) echo arm64 > /musl-arch; echo aarch64-unknown-linux-musl > /rust-target ;; \
        *) echo "unsupported TARGETPLATFORM=${TARGETPLATFORM}" >&2; exit 1 ;; \
    esac; \
    ARCH="$(cat /musl-arch)"; \
    echo "Docker build: TARGETPLATFORM=${TARGETPLATFORM:-} TARGETARCH=${TARGETARCH:-} -> ARCH=${ARCH}"

# aws-lc-sys (rustls) needs CMake + a C/C++ toolchain; musl-tools for *-linux-musl link.
# rust-version in Cargo.toml must stay <= image rustc (see FROM above).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    musl-tools \
    && \
    ARCH="$(cat /musl-arch)"; \
    case "$ARCH" in \
        amd64) rustup target add x86_64-unknown-linux-musl ;; \
        arm64) rustup target add aarch64-unknown-linux-musl ;; \
        *) echo "unsupported ARCH=$ARCH" >&2; exit 1 ;; \
    esac && \
    rm -rf /var/lib/apt/lists/*

# Point aws-lc-sys / cc-rs at the arch-correct musl GCC (matches CI musl-tools layout).
RUN set -eu; \
    ARCH="$(cat /musl-arch)"; \
    case "$ARCH" in \
        amd64) \
            printf '%s\n' \
                'export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc' \
                'export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc' \
                > /etc/musl-cargo-env.sh \
            ;; \
        arm64) \
            printf '%s\n' \
                'export CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc' \
                'export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc' \
                > /etc/musl-cargo-env.sh \
            ;; \
        *) echo "unsupported ARCH=$ARCH" >&2; exit 1 ;; \
    esac

WORKDIR /usr/src/app

# Create a dummy project to cache dependencies
RUN cargo init --bin .

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Build only the dependencies for the specified target
# This step is cached as long as Cargo.toml/Cargo.lock don't change.
RUN . /etc/musl-cargo-env.sh && \
    RUST_TARGET="$(cat /rust-target)" && \
    cargo build --release --target "$RUST_TARGET"

# Now, copy the actual application source code and build it
RUN rm src/*.rs
COPY src ./src
# Embedded at compile time via include_str! in src/i18n.rs (../locales/...)
COPY locales ./locales
RUN . /etc/musl-cargo-env.sh && \
    RUST_TARGET="$(cat /rust-target)" && \
    cargo build --release --target "$RUST_TARGET"

# Create a symlink to the final build artifact with a predictable name
# This solves the problem of the COPY command in the next stage not being able to use logic.
RUN RUST_TARGET="$(cat /rust-target)" && \
    ln -s "/usr/src/app/target/${RUST_TARGET}" /usr/src/app/target/final_target

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
