#!/bin/bash
set -e

echo "=========================================="
echo "Project Development Environment"
echo "=========================================="
echo ""

# Display Rust version info
echo "Rust Toolchain:"
rustc --version
cargo --version
echo ""

# Display available tools
echo "Available Development Tools:"
echo "  ✓ cargo clippy      - Linting and security checks"
echo "  ✓ cargo fmt         - Code formatting"
echo "  ✓ cargo audit       - Security vulnerability scanning"
echo "  ✓ cargo watch       - Auto-rebuild on file changes"
echo "  ✓ cargo tree        - Dependency tree visualization"
echo "  ✓ cargo outdated    - Check for outdated dependencies"
echo "  ✓ cargo bloat       - Binary size analysis"
echo ""

# Display workspace info
echo "Workspace Structure:"
echo "  ├── compiler/           - Compiler crate (lib + binary)"
echo "  ├── scanner-sdk/        - Scanner SDK (traits + core)"
echo "  └── scanners/           - Scanner implementations"
echo ""

echo "Note: cargo-deny and cargo-geiger require Rust 1.85+"
echo "Your deny.toml will be validated in CI/CD with GitHub Actions"
echo ""

# Quick health check
echo "Workspace Health Check:"
if cargo check --workspace --all-targets 2>&1 | grep -q "Finished"; then
    echo "  ✓ All crates compile successfully"
else
    echo "  ⚠ Some crates have compilation issues (run 'cargo check' for details)"
fi
echo ""

echo "Ready for development! 🦀"
echo ""
echo "Quick Start:"
echo "  cargo build              - Build all crates"
echo "  cargo test               - Run all tests"
echo "  cargo clippy --all       - Lint all code"
echo "  cargo run -p compiler    - Run compiler"
echo ""