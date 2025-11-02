# ESP Compiler - Rust Development Container
# Optimized for secure Rust development with essential tooling

FROM rust:1.83-bookworm

# Set working directory
WORKDIR /workspace

# Set environment variables for Rust tooling
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_BACKTRACE=1 \
    CARGO_INCREMENTAL=1

# Install system dependencies
RUN apt-get update && apt-get install -y \
    # Build essentials
    build-essential \
    pkg-config \
    libssl-dev \
    # Development tools
    git \
    curl \
    wget \
    # Debugging tools
    gdb \
    lldb \
    # Utilities
    vim \
    nano \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain components
RUN rustup component add \
    clippy \
    rustfmt \
    rust-src \
    rust-analysis

# Install essential Cargo tools (split into groups to handle compatibility)
# Tools that work with current stable Rust
RUN cargo install --locked \
    cargo-audit \
    cargo-outdated \
    cargo-watch \
    cargo-tree

# Install additional security and analysis tools
RUN cargo install --locked cargo-bloat

# Note: Several popular cargo tools now require Rust 1.85+ due to edition2024:
# - cargo-deny (dependency policy) - use CI/CD with GitHub Actions
# - cargo-geiger (unsafe detection) - requires Rust 1.85+
# - cargo-expand (macro expansion) - requires edition2024
# - cargo-edit (dependency management) - requires edition2024
# These can be added when Rust 1.85+ is released, or run in CI/CD with latest Rust

# Create post-create script directory
RUN mkdir -p /workspace/.devcontainer

# Create a basic post-create script
RUN echo '#!/bin/bash\n\
echo "Rust Development Environment"\n\
echo "===================================="\n\
rustc --version\n\
cargo --version\n\
echo ""\n\
echo "Available tools:"\n\
echo "  - cargo clippy    (linting)"\n\
echo "  - cargo fmt       (formatting)"\n\
echo "  - cargo audit     (security auditing)"\n\
echo "  - cargo watch     (auto-rebuild)"\n\
echo "  - cargo tree      (dependency tree)"\n\
echo "  - cargo outdated  (outdated deps check)"\n\
echo "  - cargo bloat     (binary size analysis)"\n\
echo ""\n\
echo "Note: cargo-deny and cargo-geiger require Rust 1.85+"\n\
echo "Use deny.toml validation in CI/CD with GitHub Actions"\n\
echo ""\n\
echo "Ready for development!"\n\
' > /workspace/.devcontainer/post-create.sh && chmod +x /workspace/.devcontainer/post-create.sh

# Set up Git configuration template (user should override)
RUN git config --global init.defaultBranch main && \
    git config --global core.autocrlf input && \
    git config --global pull.rebase false

# Health check (optional)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD rustc --version || exit 1

# Default command
CMD ["/bin/bash"]