# Endpoint State Policy (ESP)

**A declarative policy language and compliance scanner for endpoint security validation**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

---

## Overview

The Endpoint State Policy (ESP) project provides a complete framework for defining and validating security compliance policies on endpoints. It consists of three main components:

1. **ESP Compiler** - Parses and validates `.esp` policy definitions
2. **ESP Scanner Base** - Core scanning framework and execution engine
3. **ESP Scanner SDK** - Reference implementation with file system, package, and service scanners

### What is ESP?

ESP is a declarative policy language that enables security teams to define **what should be true** about their endpoints without writing imperative code. Policies are written in `.esp` files and executed by scanners to produce compliance reports.

**Example ESP Policy:**

```esp
META
    version `1.0.0`
    esp_version `1.0`
    author `ESP-Test-Suite`
    date `2025-11-04`
    severity `high`
    platform `linux`
    description `SET operations test using OBJECT_REF pattern`
    control_framework `TEST`
    control `SET-OPS-FINAL`
    esp_scan_id `set-ops-object-ref`
    criticality `high`
    tags `linux,set-operations,validation,corrected`
META_END

DEF
    # ========================================================================
    # VARIABLES
    # ========================================================================

    VAR passwd_path string `scanfiles/passwd`
    VAR shadow_path string `scanfiles/shadow`
    VAR sshd_config_path string `scanfiles/sshd_config`
    VAR sudoers_path string `scanfiles/sudoers`
    VAR hosts_allow_path string `scanfiles/hosts.allow`
    VAR hosts_deny_path string `scanfiles/hosts.deny`

    VAR required_owner_uid string `0`
    VAR required_group_gid string `0`

    # ========================================================================
    # STATES
    # ========================================================================

    STATE file_exists
        exists boolean = true
    STATE_END

    STATE file_readable
        exists boolean = true
        readable boolean = true
    STATE_END

    STATE secure_ownership
        file_owner string = VAR required_owner_uid
        file_group string = VAR required_group_gid
        exists boolean = true
    STATE_END

    # ========================================================================
    # OBJECTS - Actual files
    # ========================================================================

    OBJECT passwd_file
        path VAR passwd_path
    OBJECT_END

    OBJECT shadow_file
        path VAR shadow_path
    OBJECT_END

    OBJECT sshd_config_file
        path VAR sshd_config_path
    OBJECT_END

    OBJECT sudoers_file
        path VAR sudoers_path
    OBJECT_END

    OBJECT hosts_allow_file
        path VAR hosts_allow_path
    OBJECT_END

    OBJECT hosts_deny_file
        path VAR hosts_deny_path
    OBJECT_END

    # ========================================================================
    # SET OPERATIONS
    # ========================================================================

    SET critical_system_files union
        OBJECT_REF passwd_file
        OBJECT_REF shadow_file
    SET_END

    SET security_config_files union
        OBJECT_REF sshd_config_file
        OBJECT_REF sudoers_file
        OBJECT_REF hosts_allow_file
    SET_END

    SET access_control_files union
        OBJECT_REF hosts_allow_file
        OBJECT_REF hosts_deny_file
    SET_END

    SET all_security_files union
        SET_REF critical_system_files
        SET_REF security_config_files
        SET_REF access_control_files
    SET_END

    SET group_a union
        OBJECT_REF passwd_file
        OBJECT_REF sshd_config_file
        OBJECT_REF sudoers_file
    SET_END

    SET group_b union
        OBJECT_REF sshd_config_file
        OBJECT_REF sudoers_file
        OBJECT_REF hosts_allow_file
    SET_END

    SET common_files intersection
        SET_REF group_a
        SET_REF group_b
    SET_END

    SET all_configs union
        OBJECT_REF sshd_config_file
        OBJECT_REF sudoers_file
        OBJECT_REF hosts_allow_file
    SET_END

    SET exclude_configs union
        OBJECT_REF sudoers_file
    SET_END

    SET remaining_configs complement
        SET_REF all_configs
        SET_REF exclude_configs
    SET_END

    # ========================================================================
    # GLOBAL OBJECTS - Containers for SET_REF
    # These can be referenced with OBJECT_REF in CTN
    # ========================================================================

    OBJECT critical_files_container
        SET_REF critical_system_files
    OBJECT_END

    OBJECT security_configs_container
        SET_REF security_config_files
    OBJECT_END

    OBJECT all_files_container
        SET_REF all_security_files
    OBJECT_END

    OBJECT common_files_container
        SET_REF common_files
    OBJECT_END

    OBJECT remaining_configs_container
        SET_REF remaining_configs
    OBJECT_END

    # ========================================================================
    # TEST 1: Basic UNION via OBJECT_REF
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST all all
            STATE_REF file_exists
            OBJECT_REF critical_files_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 2: UNION with Three Operands
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST any all
            STATE_REF file_exists
            OBJECT_REF security_configs_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 3: Nested UNION (3 SET_REF deep)
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST any all
            STATE_REF file_readable
            OBJECT_REF all_files_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 4: INTERSECTION Operation
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST all all
            STATE_REF file_exists
            OBJECT_REF common_files_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 5: COMPLEMENT Operation
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST all all
            STATE_REF file_exists
            OBJECT_REF remaining_configs_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 6: Mixed OBJECT_REF - Direct and Container
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST any all
            STATE_REF file_exists
            OBJECT_REF passwd_file
            OBJECT_REF security_configs_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 7: Multiple Container References
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST any all
            STATE_REF file_exists
            OBJECT_REF critical_files_container
            OBJECT_REF security_configs_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 8: OR Criteria with Containers
    # ========================================================================

    CRI OR
        CTN file_metadata
            TEST all all
            STATE_REF secure_ownership
            OBJECT_REF critical_files_container
        CTN_END

        CTN file_metadata
            TEST all all
            STATE_REF file_readable
            OBJECT_REF security_configs_container
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 9: Local Object with SET_REF (Alternative Pattern)
    # ========================================================================

    CRI AND
        CTN file_metadata
            TEST all all
            STATE_REF file_exists
            OBJECT local_set_ref
                SET_REF critical_system_files
            OBJECT_END
        CTN_END
    CRI_END

    # ========================================================================
    # TEST 10: Chained Operations
    # ========================================================================

    SET step1 union
        SET_REF critical_system_files
        SET_REF access_control_files
    SET_END

    SET step2 complement
        SET_REF step1
        SET_REF security_config_files
    SET_END

    OBJECT chained_container
        SET_REF step2
    OBJECT_END

    CRI AND
        CTN file_metadata
            TEST any all
            STATE_REF file_exists
            OBJECT_REF chained_container
        CTN_END
    CRI_END

DEF_END

```

---

## Features

### 🎯 **Policy Language (ESP)**
- **Declarative syntax** - Define desired state, not procedures
- **Type-safe** - Variables with data types (string, int, boolean, float)
- **Composable** - Reusable variables, objects, states, and filters
- **Expressive** - Pattern matching, set operations, runtime computations

### 🔍 **Scanning Framework**
- **Pluggable architecture** - Easy to add new scanner types
- **Three-phase validation** - Existence → State → Item checks
- **Behavior system** - Configure scanner behavior at runtime
- **Batch optimization** - Efficient multi-object collection

### 📊 **Compliance Reporting**
- **SIEM-ready JSON output** - Structured compliance findings
- **Detailed diagnostics** - Clear pass/fail reasons
- **Tree-based logic** - AND/OR/NOT policy composition
- **Audit trail** - Full execution metadata

### 🛡️ **Security First**
- **No code injection** - Policies are data, not code
- **Whitelisted commands** - Secure command execution
- **Input validation** - Type-checked at compile time
- **Principle of least privilege** - Minimal permissions required

---

## Repository Structure

```
Endpoint-State-Policy/
├── esp_compiler/           # ESP language compiler
│   ├── src/
│   │   ├── grammar/        # AST definitions and parser
│   │   ├── validator/      # Semantic validation
│   │   └── logging/        # Structured logging
│   └── Cargo.toml
│
├── esp_scanner_base/       # Core scanning framework
│   ├── src/
│   │   ├── execution/      # Execution engine
│   │   ├── resolution/     # Variable and reference resolution
│   │   ├── strategies/     # CTN traits and contracts
│   │   ├── types/          # Core type system
│   │   └── results/        # Result generation
│   └── Cargo.toml
│
├── esp_scanner_sdk/        # Reference scanner implementations
│   ├── src/
│   │   ├── contracts/      # CTN contracts
│   │   ├── collectors/     # Data collectors
│   │   ├── executors/      # Validation executors
│   │   ├── commands/       # Command configurations
│   │   ├── lib.rs          # Registry creation
│   │   └── main.rs         # CLI application
│   └── Cargo.toml
│
├── docs/                   # Additional documentation
├── Cargo.toml              # Workspace configuration
├── Makefile                # Build automation
├── LICENSE                 # Apache 2.0 license
└── README.md               # This file
```

---

## Quick Start

### Prerequisites

- **Rust 1.70+** (stable toolchain)
- **Cargo** (included with Rust)

Install Rust from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/CurtisSlone/Endpoint-State-Policy.git
cd Endpoint-State-Policy

# Build all workspace crates
make build

# Or build with cargo directly
cargo build --workspace --release
```

### Run a Scan

```bash
# Run the scanner CLI
cargo run --release --bin scanner -- scan examples/sudo_priv_check.esp

# Or install and use directly
make install
scanner scan examples/sudo_priv_check.esp
```

### Run Tests

```bash
# Run all tests
make test

# Run specific crate tests
cargo test -p esp_compiler
cargo test -p esp_scanner_base
cargo test -p esp_scanner_sdk

# Run with verbose output
cargo test -- --nocapture
```

---

## Components

### 1. ESP Compiler

The compiler parses `.esp` files into a validated Abstract Syntax Tree (AST).

**Key Features:**
- Lexical analysis and parsing
- Semantic validation
- Type checking
- Reference resolution
- Error reporting with source locations

**Usage:**

```rust
use esp_compiler::Parser;

let source = std::fs::read_to_string("policy.esp")?;
let parser = Parser::new(&source);
let ast = parser.parse()?;

// AST is now available for execution
```

**See:** `esp_compiler/` for implementation details

---

### 2. ESP Scanner Base

The core framework that executes ESP policies and generates compliance reports.

**Key Components:**

- **Execution Engine** - Orchestrates policy execution
- **Resolution Engine** - Resolves variables, filters, and set operations
- **Strategy Registry** - Maps CTN types to implementations
- **Result Generator** - Produces SIEM-ready JSON reports

**CTN (Criterion Type Node) System:**

Each CTN type requires three components:

1. **Contract** - Defines interface requirements
2. **Collector** - Gathers system data
3. **Executor** - Validates data against policy

**Usage:**

```rust
use esp_scanner_base::strategies::CtnStrategyRegistry;
use esp_scanner_base::execution::ExecutionEngine;

// Create registry and register strategies
let registry = create_scanner_registry()?;

// Execute policy
let mut engine = ExecutionEngine::new(context, registry);
let result = engine.execute()?;

// result is a ScanResult with full compliance details
```

**See:** `esp_scanner_base/` for framework details

---

### 3. ESP Scanner SDK

Reference implementation with production-ready scanners.

**Included Scanners:**

| CTN Type | Description | Example Use Case |
|----------|-------------|------------------|
| `file_metadata` | File permissions, owner, group | Permission audits |
| `file_content` | File content validation | Configuration checks |
| `json_record` | JSON structure validation | Config file validation |
| `rpm_package` | RPM package verification | Installed package checks |
| `systemd_service` | Systemd service status | Service state validation |
| `sysctl_parameter` | Kernel parameters | Kernel security settings |
| `selinux_status` | SELinux enforcement mode | SELinux compliance |

**Usage:**

```rust
use esp_scanner_sdk::create_scanner_registry;

// Create registry with all scanners
let registry = create_scanner_registry()?;

// Registry includes all CTN types above
```

**See:**
- `esp_scanner_sdk/docs/ESP_Scanner_SDK_Development_Guide.md` - Complete development guide
- `esp_scanner_sdk/src/` - Reference implementations

---

## Development

### Building

```bash
# Development build (fast, with debug symbols)
make dev

# Release build (optimized, no debug symbols)
make release

# Check compilation without building
make check
```

### Testing

```bash
# Run all tests
make test

# Run only unit tests
make test-unit

# Run with all features enabled
make test-all

# Run in watch mode (requires cargo-watch)
make watch-test
```

### Code Quality

```bash
# Format code
make format

# Check formatting
make format-check

# Run clippy lints
make lint

# Auto-fix lint issues
make lint-fix
```

### Security Auditing

```bash
# Run all security checks
make security

# Check for known vulnerabilities
make audit

# Check dependency policies (requires Rust 1.85+)
make deny
```

### Documentation

```bash
# Generate and open documentation
make docs

# Generate all documentation (including private items)
make docs-all
```

### CI/CD Pipeline

```bash
# Run all CI checks (format, lint, test, security)
make ci

# Run pre-commit checks (faster subset)
make pre-commit
```

---

## Creating Custom Scanners

The SDK is designed to be extended with custom scanners. Follow the three-component pattern:

### 1. Create a Contract

```rust
// contracts/my_scanner.rs
use esp_scanner_base::strategies::*;

pub fn create_my_scanner_contract() -> CtnContract {
    let mut contract = CtnContract::new("my_scanner".to_string());

    // Define object and state requirements
    // Configure field mappings
    // Set collection strategy

    contract
}
```

### 2. Implement a Collector

```rust
// collectors/my_collector.rs
use esp_scanner_base::strategies::*;

pub struct MyCollector { /* ... */ }

impl CtnDataCollector for MyCollector {
    fn collect_for_ctn_with_hints(/* ... */) -> Result<CollectedData> {
        // Gather system data
    }
}
```

### 3. Implement an Executor

```rust
// executors/my_executor.rs
use esp_scanner_base::strategies::*;

pub struct MyExecutor { /* ... */ }

impl CtnExecutor for MyExecutor {
    fn execute_with_contract(/* ... */) -> Result<CtnExecutionResult> {
        // Validate collected data
    }
}
```

### 4. Register Your Scanner

```rust
// lib.rs
pub fn create_scanner_registry() -> Result<CtnStrategyRegistry> {
    let mut registry = CtnStrategyRegistry::new();

    registry.register_ctn_strategy(
        Box::new(MyCollector::new()),
        Box::new(MyExecutor::new(create_my_scanner_contract())),
    )?;

    Ok(registry)
}
```

**Complete Guide:** See `esp_scanner_sdk/docs/ESP_Scanner_SDK_Development_Guide.md`

---

## ESP Language Reference

### Policy Structure

```esp
DEFINITION policy_name
MODULE esp 1.0.0

METADATA
    title `Policy Title`
    description `Policy description`
    version `1.0.0`
METADATA_END

# Variables
VAR variable_name type `value`

# Objects (what to check)
OBJECT object_name
    field_name VAR variable_name
OBJECT_END

# States (desired conditions)
STATE state_name
    field_name type operation value
STATE_END

# Filters (object selection)
FILTER filter_name
    STATE_REF state_name
    ACTION include|exclude
FILTER_END

# Sets (object groups)
SET set_name
    OBJECT object1
    OBJECT object2
SET_END

# Runtime computations
RUN computation_name
    PARAM param_name type value
    OPERATION operation_type param1 param2
    OUTPUT variable_name
RUN_END

# Criteria (validation rules)
CTN ctn_type
    BEHAVIOR behavior_name param value
    TEST existence_check item_check
    STATE_REF state_name
    OBJECT_REF object_name
CTN_END
```

### Supported Operations

**String Operations:**
- `=`, `!=` - Equality
- `contains`, `not_contains` - Substring matching
- `starts_with`, `ends_with` - Prefix/suffix matching
- `pattern_match` - Regex pattern matching
- `case_insensitive_equals` - Case-insensitive comparison

**Numeric Operations:**
- `=`, `!=` - Equality
- `>`, `<`, `>=`, `<=` - Comparison

**Boolean Operations:**
- `=`, `!=` - Equality

### Test Specifications

```esp
TEST <existence_check> <item_check>
```

**Existence Checks:**
- `all` - All expected objects must exist
- `any` - At least one expected object must exist
- `none` - No objects should exist
- `exact N` - Exactly N objects must exist
- `at_least N` - At least N objects must exist

**Item Checks:**
- `all` - All objects must pass state validation
- `any` - At least one object must pass
- `none` - No objects should pass
- `at_least_one` - At least one must pass

---

## Output Format

Scan results are generated in SIEM-ready JSON format:

```json
{
  "scan_id": "scan_1234567890",
  "metadata": {
    "title": "Sudoers Security Configuration",
    "description": "Validates sudoers file permissions and content",
    "version": "1.0.0",
    "module": "esp 1.0.0"
  },
  "host": {
    "hostname": "server01",
    "os": "Linux",
    "architecture": "x86_64"
  },
  "results": {
    "passed": false,
    "check": {
      "total_criteria": 2,
      "passed_criteria": 1,
      "failed_criteria": 1,
      "pass_percentage": 50.0,
      "status": "non_compliant"
    },
    "findings": [
      {
        "finding_id": "finding_001",
        "criterion_type": "file_content",
        "status": "fail",
        "severity": "high",
        "message": "Content validation failed",
        "details": { /* ... */ }
      }
    ]
  },
  "timestamps": {
    "scan_start": "2025-01-15T10:30:00Z",
    "scan_end": "2025-01-15T10:30:05Z"
  }
}
```

---

## Configuration

### Workspace Configuration

The project uses Cargo workspace features for dependency management:

```toml
# Shared dependencies
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
```

### Build Profiles

Multiple build profiles are available:

- **`dev`** - Fast compilation, debug symbols
- **`release`** - Optimized for performance
- **`release-with-debug`** - Optimized but with debug symbols
- **`security`** - Maximum security (LTO, panic=abort)

```bash
# Build with specific profile
cargo build --profile security
```

### Security Configuration

The project includes security-focused tooling:

- **`clippy.toml`** - Security-focused lint configuration
- **`deny.toml`** - Dependency policy enforcement
- **`rustfmt.toml`** - Code formatting standards

---

## Performance

### Benchmarks

Typical performance characteristics:

| Operation | Time | Memory |
|-----------|------|--------|
| Parse ESP file (1KB) | ~2ms | <1MB |
| File metadata scan | ~5ms per file | ~1MB |
| File content scan | ~50ms per file | ~10MB |
| RPM package batch | ~100ms for 100 packages | ~5MB |
| JSON validation | ~10ms per file | ~10MB |

### Optimization Tips

1. **Use batch collection** for command-based scanners
2. **Enable behaviors** only when needed
3. **Use filters** to reduce object count before collection
4. **Prefer metadata checks** over content checks when possible

---

## Troubleshooting

### Common Issues

**Issue:** `error: could not compile 'esp_compiler'`
- **Solution:** Ensure Rust 1.70+ is installed: `rustc --version`

**Issue:** Test failures with file not found
- **Solution:** Create test files: `mkdir -p scanfiles && echo "test" > scanfiles/test.txt`

**Issue:** `cargo deny` fails
- **Solution:** Requires Rust 1.85+. Run `rustup update` or skip in CI

**Issue:** Permission denied errors during scan
- **Solution:** Some scanners require elevated privileges. Run with `sudo` if needed.

### Debug Mode

Enable verbose logging:

```bash
RUST_LOG=debug cargo run --bin scanner -- scan policy.esp

# Or with installed binary
RUST_LOG=debug scanner scan policy.esp
```

---

## Contributing

Contributions are welcome! Please:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### Development Guidelines

- Run `make pre-commit` before committing
- Add tests for new features
- Update documentation
- Follow existing code style
- Run `make ci` to verify all checks pass

---

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)

Copyright (c) 2025 Curtis Slone

---

## Roadmap

### Version 1.0 (Current)
- ✅ Core ESP language and compiler
- ✅ Execution engine with full feature support
- ✅ Reference scanner implementations
- ✅ Pattern matching support
- ✅ BEHAVIOR system
- ✅ SET, FILTER, RUN operations

### Version 1.1 (Planned)
- [ ] Binary file content validation
- [ ] Recursive directory scanning
- [ ] Performance metrics collection
- [ ] Enhanced error messages
- [ ] Additional scanner types

### Version 2.0 (Future)
- [ ] Windows registry support
- [ ] Network resource scanning
- [ ] Database query support
- [ ] Remote endpoint scanning
- [ ] Policy composition and inheritance

---

## Resources

### Documentation
- [ESP Language Specification](docs/EBNF.md)
- [Scanner SDK Development Guide](esp_scanner_sdk/docs/ESP_Scanner_SDK_Development_Guide.md)
- [API Documentation](https://docs.rs/esp_scanner_base) (generated with `make docs`)

### Examples
- [Example Policies](examples/)
- [Test Cases](esp_scanner_sdk/tests/)

### Community
- **Repository**: https://github.com/CurtisSlone/Endpoint-State-Policy
- **Issues**: https://github.com/CurtisSlone/Endpoint-State-Policy/issues
- **Author**: Curtis Slone (curtis@scanset.io)

---

## Acknowledgments

Built with:
- **Rust** - Systems programming language
- **Serde** - Serialization framework
- **Regex** - Pattern matching
- **Thiserror** - Error handling

---

## Security

For security issues, please email curtis@scanset.io directly rather than opening a public issue.

Security features:
- ✅ No code execution from ESP files
- ✅ Whitelisted command execution
- ✅ Type-safe compilation
- ✅ Dependency vulnerability scanning
- ✅ NIST SP 800-218 compliance

---

**ESP - Making endpoint compliance declarative, testable, and automatable.**
