# Endpoint State Policy (ESP)

**A declarative policy language and compliance scanner for endpoint security validation**

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

---
# Overview

The Endpoint State Policy (ESP) project provides a complete framework for defining and executing declarative, machine-verifiable security controls on endpoints.

ESP is intentionally designed as a replacement for SCAP/XCCDF-style content, while preserving the core idea that matters: expressing security intent as executable policy, not imperative scripts.

The repository is organized as a workspace containing three primary components:

- **ESP Compiler** — Parses and validates `.esp` policy definitions into a typed AST
- **ESP Scanner Base** — A generic execution engine that evaluates policies against collected system state
- **ESP Scanner SDK** — Reference collectors and executors for common endpoint resources (files, services, packages, sysctl, etc.)

This separation allows ESP to be used in multiple ways:

- As a standalone open-source scanner
- As a policy execution engine embedded into another platform
- As a portable policy format executed by multiple scanner implementations

---

## What Is ESP?

ESP is a declarative policy language for endpoint security validation.

Instead of writing scripts that perform checks, ESP allows you to describe:

- What data should be collected
- What conditions must be true
- How those conditions are combined
- When findings should be reported

Policies are written as data, not code. The scanner is responsible for execution.

At a high level, ESP policies are composed of:

- **Objects** — Things to observe (files, services, registry keys, packages, etc.)
- **States** — Assertions about object properties
- **Criteria (CTNs / CRIs)** — Logical evaluation of states across objects
- **Sets & Filters** — Collection and reduction of object groups
- **Runtime Operations** — Deterministic computation (math, string ops, extraction)

This design makes ESP:

- Deterministic
- Inspectable
- Testable
- Extensible

---

## What Problem ESP Solves

Traditional compliance automation (e.g., SCAP/XCCDF) suffers from:

- Verbose, fragile XML
- Tight coupling between content and execution
- Poor extensibility
- High authoring and maintenance cost

ESP addresses this by:

- Separating policy definition from execution
- Using a typed, validated DSL
- Enforcing clear contracts between collectors and executors
- Making policy authoring readable by humans and machines

ESP focuses exclusively on **technical controls** — controls that can be validated by inspecting endpoint state.

---

## Control Framework Coverage

ESP can express and execute technical controls from any control framework that defines observable system requirements, including:

- NIST SP 800-53
- NIST SP 800-171
- MITRE ATT&CK (detection-oriented techniques)
- DISA STIGs
- CIS Benchmarks
- Organization- or program-specific baselines

ESP does not attempt to model governance, policy, or process controls. It is explicitly designed to answer:

> "Is this endpoint in the required technical state?"

---

## Example: Translating a Technical Control into ESP

**Framework:** NIST SP 800-53  
**Control:** CM-6 – Configuration Settings

> "The organization configures systems to enforce least functionality by restricting unnecessary or insecure configuration settings."

A common Linux implementation of this control includes:

- SSH root login must be disabled
- SSH configuration file must be owned by root
- Insecure SSH options must not be present

Below is how that control intent is expressed directly in ESP.

### ESP Representation

```esp
META
    control_framework `NIST-800-53`
    control `CM-6`
    description `Ensure SSH is securely configured`
META_END

DEF

# Object: SSH configuration file
OBJECT ssh_config
    path `/etc/ssh`
    filename `sshd_config`

    select record
        content text
        owner uid
    select_end
OBJECT_END

# States: Required security properties
STATE no_root_login
    content string not_contains `PermitRootLogin yes`
STATE_END

STATE secure_owner
    owner string = `0`
STATE_END

# Criteria: All conditions must hold
CRI AND
    CTN ssh_hardening
        TEST all all
        STATE_REF no_root_login
        STATE_REF secure_owner
        OBJECT_REF ssh_config
    CTN_END
CRI_END

DEF_END
```

This policy:

- Collects the SSH configuration file
- Evaluates its content and ownership
- Produces a deterministic pass/fail result
- Can be executed by any ESP-compatible scanner

---

## From Policy to Execution

Once compiled, ESP policies are:

1. Validated for structure, types, and references
2. Converted into an executable AST
3. Evaluated by the scanner using registered collectors and executors
4. Emitted as structured, SIEM-ready compliance results

The rest of this repository shows:

- How the language is defined
- How execution is orchestrated
- How to extend the system with new scanners

The example below demonstrates a full ESP policy in context.

**Example ESP Policy:**

```esp
META
    version `1.0.0`
    esp_version `1.0`
    author `DISA-CISA-MITRE`
    control_framework `MITRE_ATTCK`
    control `T1053.005`
    description `Detect malicious cron persistence`
META_END

DEF

# Variables
VAR system_crontab string `/etc/crontab`
VAR cron_dirs string `/etc/cron.d`
VAR suspicious_cmds string `curl|wget|nc|bash -i|base64`

# Runtime - Calculate 30-day threshold
RUN age_threshold ARITHMETIC
    literal 30
    * 86400
RUN_END

# Objects
OBJECT system_cron
    path `/etc`
    filename `crontab`

    select record
        content text
        modified_time mtime
    select_end
OBJECT_END

OBJECT cron_directory
    path VAR cron_dirs
    filename `*`
    behavior recurse false

    select record
        content text
    select_end
OBJECT_END

OBJECT user_crons
    path `/var/spool/cron`
    filename `*`

    select record
        content text
        owner uid
    select_end
OBJECT_END

# States
STATE no_suspicious_content
    content string not_contains `curl`
    content string not_contains `wget`
    content string not_contains `bash -i`
STATE_END

STATE secure_ownership
    owner string = `0`
STATE_END

STATE recently_modified
    modified_time int >= VAR age_threshold
STATE_END

# Sets - Aggregate all cron locations
SET all_cron_files union
    OBJECT_REF system_cron
    OBJECT_REF cron_directory
    OBJECT_REF user_crons
    FILTER include
        STATE_REF no_suspicious_content
    FILTER_END
SET_END

# Criteria - Detection logic
CRI AND
    # Check system crontab
    CTN system_check
        TEST all all
        STATE_REF no_suspicious_content
        STATE_REF secure_ownership
        OBJECT_REF system_cron
    CTN_END

    # Check for recent suspicious changes
    CRI NOT
        CTN detect_suspicious_mods
            TEST any all
            STATE suspicious_and_recent
                content string pattern_match VAR suspicious_cmds
                modified_time int >= VAR age_threshold
            STATE_END
            OBJECT set_check
                SET_REF all_cron_files
            OBJECT_END
        CTN_END
    CRI_END
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
cargo run -- esp/sudo_priv_check.esp

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
# Microsoft Entra MFA Compliance Verification
# Verify user accounts require Multi-Factor Authentication

META
version `1.0.0`
ics_version `1.0`
author `compliance-team`
date `2024-01-20`
severity `high`
platform `microsoft-entra`
description `Verify user accounts require MFA through Conditional Access policies`
compliance_framework `Security-Controls`
tags `mfa,conditional-access,security,microsoft-entra`
META_END

DEF
# Variables for MFA policy validation
VAR admin_center_url string `https://entra.microsoft.com`
VAR conditional_access_path string `Identity/Protection/ConditionalAccess`
VAR policy_required_state string `On`
VAR users_scope_required string `All users included`

# Global state for MFA policy validation
STATE mfa_policy_state_check
    policy_state string = VAR policy_required_state
    policy_enabled boolean = true
STATE_END

STATE mfa_user_scope_check
    users_included string = VAR users_scope_required
    all_users_covered boolean = true
STATE_END

STATE mfa_exclusion_validation
    exclusions_documented boolean = true
    ao_approval_required boolean = true
STATE_END

# Global objects for different validation steps
OBJECT entra_admin_center
    url VAR admin_center_url
    access_path VAR conditional_access_path
    required_role `Conditional Access Administrator`

    parameters string
        authentication_method `admin_credentials`
        session_timeout `3600`
    parameters_end
OBJECT_END

OBJECT conditional_access_policies
    navigation_path `Identity >> Protection >> Conditional Access`
    policy_section `Policies`

    select string
        PolicyName `MFA Policy Name`
        PolicyState `Policy State`
        UsersScope `Users Configuration`
        Exclusions `Excluded Users`
    select_end

    behavior verify_policy_existence check_state validate_scope
OBJECT_END

OBJECT mfa_policy_configuration
    policy_type `MFA Enforcement`

    parameters record_data
        PolicyState `On`
        UsersIncluded `All users included`
        ExclusionDocumentation `Required with AO approval`
        ComplianceValidation `Mandatory`
    parameters_end
OBJECT_END

# Criteria for MFA compliance verification
CRI AND

# Step 1-3: Access admin center and navigate to Conditional Access
CTN admin_center_access
    TEST any all AND
    STATE_REF mfa_policy_state_check
    OBJECT_REF entra_admin_center

    # Local validation for admin access
    STATE admin_access_validation
        admin_role string contains `Conditional Access Administrator`
        access_granted boolean = true
        navigation_successful boolean = true
    STATE_END
CTN_END

# Step 4: Verify policy state is "On"
CTN policy_state_verification
    TEST all all AND
    STATE_REF mfa_policy_state_check
    OBJECT_REF conditional_access_policies

    # Local state for policy state validation
    STATE policy_state_validation
        state_value string = `On`
        state_verified boolean = true
        finding_if_off boolean = true
    STATE_END
CTN_END

# Step 5: Verify "All users included" is specified
CTN user_scope_verification
    TEST all all AND
    STATE_REF mfa_user_scope_check
    OBJECT_REF mfa_policy_configuration

    # Local state for user scope validation
    STATE scope_validation
        users_setting string = `All users included`
        scope_verified boolean = true
        complete_coverage boolean = true
    STATE_END
CTN_END

# Step 6: Verify exclusions are documented with AO
CTN exclusion_documentation_check
    TEST all all OR
    STATE_REF mfa_exclusion_validation

    # Local state for exclusion validation
    STATE exclusion_validation
        exclusions_exist boolean = false
        documentation_complete boolean = true
        ao_approval_obtained boolean = true
        compliance_satisfied boolean = true
    STATE_END

    # Local object for exclusion tracking
    OBJECT exclusion_tracker
        exclusion_list_path `Users >> Exclude`
        documentation_required `AO Authorization`

        parameters record_data
            ExclusionCount `0 or documented`
            AOApproval `Required for any exclusions`
            DocumentationStatus `Complete`
        parameters_end

        select string
            ExcludedUser `User Identity`
            ExclusionReason `Business Justification`
            AOSignature `Authorizing Official`
            ApprovalDate `Authorization Date`
        select_end
    OBJECT_END
CTN_END

CRI_END

# Nested criteria for comprehensive finding determination
CRI OR

CTN finding_determination
    TEST any all

    # Combined finding state
    STATE compliance_finding
        policy_not_on boolean != true
        users_not_included boolean != true
        exclusions_not_documented boolean != true
        finding_exists boolean = false
    STATE_END

    # Finding object
    OBJECT compliance_finding_object
        finding_criteria `MFA policy not On OR All users not included OR exclusions not documented`
        finding_severity `high`
        remediation_required true

        parameters string
            FindingType `MFA Configuration Non-Compliance`
            RequiredActions `Enable policy, include all users, document exclusions`
            ComplianceStandard `Security Controls Framework`
        parameters_end
    OBJECT_END
CTN_END

CRI_END

DEF_END

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
- [Scanner SDK Development Guide](docs/ESP_Scanner_SDK_Development_Guide.md)
- [API Documentation](esp_scanner_base) (generated with `make docs`)

### Examples
- [Example Policies](esp_scanner_sdk/esp/)
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



