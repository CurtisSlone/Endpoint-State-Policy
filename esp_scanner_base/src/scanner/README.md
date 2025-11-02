# ESP Scanner SDK

Complete implementation of file system compliance scanner for ESP (Endpoint State Policy).

## Overview

This scanner module implements CTN (Criterion Type Node) strategies for validating file system compliance against ESP definitions. It supports two CTN types:

- **`file_metadata`** - Fast metadata validation (permissions, owner, group, existence, size)
- **`file_content`** - Content-based validation with string operations

## Architecture

```
scanner/
├── contracts/           # CTN contract definitions
│   ├── mod.rs
│   └── file_contracts.rs
├── collectors/          # Data collection implementations
│   ├── mod.rs
│   └── filesystem.rs
├── executors/           # Validation logic
│   ├── mod.rs
│   ├── file_metadata.rs
│   └── file_content.rs
├── tests/
│   ├── mod.rs
│   └── integration_tests.rs
└── mod.rs              # Public API
```

## Components

### 1. CTN Contracts

**File: `contracts/file_contracts.rs`**

Defines the interface contracts for both CTN types:

- **Object requirements**: What fields the OBJECT block must provide
- **State requirements**: What fields can be validated and with which operations
- **Field mappings**: How to map between ESP field names and collected data
- **Collection strategy**: Performance hints and capabilities

### 2. FileSystemCollector

**File: `collectors/filesystem.rs`**

Implements `CtnDataCollector` trait to gather file system data:

- **Metadata mode**: Fast `stat()` operations for permissions, owner, group, size
- **Content mode**: Full file reading for string validation
- **Error handling**: Distinguishes between file not found, access denied, and collection failures
- **Platform support**: Unix-specific metadata extraction with fallbacks

### 3. Executors

#### FileMetadataExecutor

**File: `executors/file_metadata.rs`**

Validates collected metadata against state requirements:

- Supports: `=`, `!=`, `>`, `<`, `>=`, `<=` operations
- Handles: String, Boolean, and Integer comparisons
- Three-phase TEST validation: existence → state → item checks

#### FileContentExecutor

**File: `executors/file_content.rs`**

Validates file content with string operations:

- Supports: `contains`, `not_contains`, `starts`, `ends`, `pattern_match`
- Handles multiple STATE_REFs per CTN
- State operator: AND (default), OR, ONE

## Usage

### Basic Example

```rust
use esp_scanner_base::api::*;
use esp_scanner_base::scanner::create_file_scanner_registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry with file strategies
    let registry = create_file_scanner_registry()?;

    // Create scanner
    let mut scanner = EspScanner::new(registry)?;

    // Scan ESP file
    let result = scanner.scan_file("sudo_priv_check.esp")?;

    // Check compliance
    println!("Status: {}", if result.results.passed {
        "COMPLIANT"
    } else {
        "NON-COMPLIANT"
    });

    // Export results
    println!("{}", result.to_json()?);

    Ok(())
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_sudo_priv_check_file

# Run with output
cargo test -- --nocapture
```

### Running Example

```bash
# Build and run example
cargo run --example file_scanner_example

# Output will be saved to scan_result.json
```

## ESP File Support

### Supported Object Fields

```esp
OBJECT example_file
    path VAR file_path        # Required - resolved from VAR
    type `file`              # Optional - informational
OBJECT_END
```

### Supported State Fields (file_metadata)

```esp
STATE metadata_check
    permissions string = `0440`     # Octal format
    owner string = `root`           # UID or username
    group string = `root`           # GID or group name
    exists boolean = true           # Boolean
    readable boolean = true         # Boolean
    size int > 100                  # Integer with comparisons
STATE_END
```

### Supported State Fields (file_content)

```esp
STATE content_check
    content string contains `logfile=`           # Contains
    content string not_contains `NOPASSWD`      # Not contains
    content string starts `/usr/sbin`           # Starts with
    content string ends `.conf`                 # Ends with
STATE_END
```

### Supported TEST Specifications

```esp
CTN file_metadata
    TEST all all              # All objects, all states pass
    TEST any all              # At least one object, all states pass
    TEST all at_least_one     # All objects, at least one state passes
    # ... other combinations
CTN_END
```

## Implementation Details

### Multi-Field State Validation

From `sudo_priv_check.esp`:

```esp
STATE file_security_posture
    permissions string = VAR required_permissions
    owner string = VAR required_owner
    group string = VAR required_group
    exists boolean = true
    readable boolean = true
STATE_END
```

All 5 fields must pass for the state to pass (AND operator by default).

### Multiple STATE_REF Support

```esp
CTN file_content
    TEST all all
    STATE_REF logging_configuration
    STATE_REF password_policy_configuration
    STATE_REF secure_path_configuration
    STATE_REF dangerous_permissions_absent
    OBJECT_REF sudoers_file
CTN_END
```

All 4 STATE_REFs must pass (CRI AND semantics).

### Variable Resolution

Variables are resolved before collection:

```esp
VAR sudoers_path string `scanfiles/sudoers`

OBJECT sudoers_file
    path VAR sudoers_path    # Resolved to "scanfiles/sudoers"
OBJECT_END
```

### Error Handling

The collector distinguishes between:

- **ObjectNotFound**: File doesn't exist
- **AccessDenied**: Permission denied
- **CollectionFailed**: Other I/O errors

This allows executors to properly handle existence checks vs. permission failures.

## Test Coverage

### Integration Tests

1. **`test_sudo_priv_check_file`**: Full scan of sudo_priv_check.esp
2. **`test_file_metadata_collection`**: Metadata collection and validation
3. **`test_file_content_validation`**: Content validation with string operations
4. **`test_registry_health`**: Registry health and statistics

### Test File Requirements

The test assumes `sudo_priv_check.esp` exists and references `scanfiles/sudoers`.

Create test structure:

```bash
mkdir -p scanfiles
echo "test content" > scanfiles/sudoers
chmod 0440 scanfiles/sudoers
```

## Extending the Scanner

### Adding New CTN Types

1. **Define contract** in `contracts/`
2. **Implement collector** or reuse existing
3. **Implement executor** for validation logic
4. **Register strategy** in `create_file_scanner_registry()`

Example:

```rust
// 1. Create contract
pub fn create_new_ctn_contract() -> CtnContract {
    let mut contract = CtnContract::new("new_ctn_type".to_string());
    // ... configure contract
    contract
}

// 2. Implement executor
pub struct NewExecutor {
    contract: CtnContract,
}

impl CtnExecutor for NewExecutor {
    // ... implement trait methods
}

// 3. Register
registry.register_ctn_strategy(
    Box::new(ExistingCollector::new()),
    Box::new(NewExecutor::new(contract)),
)?;
```

## Performance Characteristics

### file_metadata

- **Collection time**: ~5ms per file
- **Memory**: ~1MB
- **Operation**: Single `stat()` syscall
- **Best for**: Permission checks, ownership validation, size checks

### file_content

- **Collection time**: ~50ms per file (varies with size)
- **Memory**: ~10MB (file size dependent)
- **Operation**: Full file read into memory
- **Best for**: Configuration validation, policy enforcement

## Platform Support

- **Unix/Linux**: Full support (permissions, owner, group)
- **Windows**: Limited support (fallback values for Unix-specific fields)
- **macOS**: Full Unix support

## Dependencies

All imports use the `crate::api::*` module which re-exports:

- `CtnDataCollector`, `CtnExecutor` traits
- `CtnContract`, `CollectedData`, `CtnExecutionResult` types
- `ExecutionContext`, `ExecutableObject`, `ExecutableState` types
- Test evaluation functions: `evaluate_existence_check`, `evaluate_item_check`, `evaluate_state_operator`

## Output Format

Scan results are SIEM-ready JSON:

```json
{
  "scan_id": "scan_sudo_priv_check_1234567890",
  "metadata": { ... },
  "results": {
    "check": {
      "total_criteria": 2,
      "passed_criteria": 1,
      "failed_criteria": 1,
      "pass_percentage": 50.0,
      "status": "non_compliant"
    },
    "findings": [ ... ],
    "passed": false
  }
}
```

## Future Enhancements

- [ ] Regex pattern matching support
- [ ] Binary file content validation
- [ ] Recursive directory scanning
- [ ] Performance metrics collection
- [ ] Windows registry support
- [ ] Network resource support
- [ ] Database query support
