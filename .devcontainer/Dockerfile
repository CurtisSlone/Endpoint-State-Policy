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

# Install essential Cargo tools
RUN cargo install --locked \
    cargo-audit \
    cargo-deny \
    cargo-outdated \
    cargo-edit \
    cargo-watch \
    cargo-expand \
    cargo-tree

# Install additional security and analysis tools
RUN cargo install --locked \
    cargo-geiger \
    cargo-bloat \
    cargo-udeps

# Create post-create script directory
RUN mkdir -p /workspace/.devcontainer

# Create a basic post-create script
RUN echo '#!/bin/bash\n\
echo "ESP Compiler Development Environment"\n\
echo "===================================="\n\
rustc --version\n\
cargo --version\n\
echo ""\n\
echo "Available tools:"\n\
echo "  - cargo clippy    (linting)"\n\
echo "  - cargo fmt       (formatting)"\n\
echo "  - cargo audit     (security auditing)"\n\
echo "  - cargo deny      (dependency policy)"\n\
echo "  - cargo watch     (auto-rebuild)"\n\
echo "  - cargo expand    (macro expansion)"\n\
echo "  - cargo geiger    (unsafe code detection)"\n\
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
