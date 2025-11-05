# ESP Scanner SDK Development Guide

**Complete guide for implementing custom compliance scanners using the ESP Scanner SDK**

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Getting Started](#getting-started)
4. [Creating a CTN Contract](#creating-a-ctn-contract)
5. [Implementing a Collector](#implementing-a-collector)
6. [Implementing an Executor](#implementing-an-executor)
7. [Registering Your Scanner](#registering-your-scanner)
8. [Advanced Features](#advanced-features)
9. [Testing Your Implementation](#testing-your-implementation)
10. [Complete Examples](#complete-examples)

---

## Overview

The ESP Scanner SDK provides a framework for implementing compliance scanners that validate system state against ESP (Endpoint State Policy) definitions. The SDK handles:

- ESP parsing and resolution
- Execution orchestration
- Result generation and reporting
- Error handling and logging

**You implement:**

- **CTN Contracts** - Define what your scanner validates
- **Collectors** - Gather data from the system
- **Executors** - Validate collected data against ESP states

---

## Architecture

### Component Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│                    ESP Definition (.esp file)               │
│  - Variables, Objects, States, Filters, Sets, Behaviors    │
└────────────────────┬────────────────────────────────────────┘
                     │ Parsed & Resolved by esp_compiler
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    Execution Context                        │
│  - Resolved variables, objects, states                     │
│  - Executable criteria tree                                │
└────────────────────┬────────────────────────────────────────┘
                     │ Executed by ExecutionEngine
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  CTN Strategy Registry                      │
│  Maps CTN types → (Collector, Executor) pairs              │
└────────────────────┬────────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────────┐    ┌──────────────────┐
│   Collector      │    │    Executor      │
│  (Your Code)     │    │  (Your Code)     │
└────────┬─────────┘    └────────┬─────────┘
         │                       │
         │ Gathers data          │ Validates data
         ▼                       ▼
┌──────────────────┐    ┌──────────────────┐
│  CollectedData   │───→│ ExecutionResult  │
└──────────────────┘    └──────────────────┘
```

### Three-Component Pattern

Every CTN type requires exactly three components:

1. **CTN Contract** (`contracts/your_ctn.rs`)
   - Defines interface requirements
   - Specifies supported operations
   - Documents field mappings
   - Declares behaviors

2. **Data Collector** (`collectors/your_collector.rs`)
   - Implements `CtnDataCollector` trait
   - Gathers system data
   - Handles behavior hints
   - Supports batch operations (optional)

3. **Executor** (`executors/your_executor.rs`)
   - Implements `CtnExecutor` trait
   - Validates collected data
   - Evaluates TEST specifications
   - Returns compliance results

---

## Getting Started

### Project Structure

```
your_scanner/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Registry creation
    ├── contracts/
    │   ├── mod.rs
    │   └── your_contract.rs      # Your CTN contract
    ├── collectors/
    │   ├── mod.rs
    │   └── your_collector.rs     # Your data collector
    ├── executors/
    │   ├── mod.rs
    │   └── your_executor.rs      # Your executor
    └── commands/                 # Optional: for command-based collectors
        ├── mod.rs
        └── platform_config.rs
```

### Dependencies

```toml
[dependencies]
esp_scanner_base = { path = "../esp_scanner_base" }
esp_compiler = { path = "../esp_compiler" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Optional: Add if using regex pattern matching
regex = "1.10"
```

---

## Creating a CTN Contract

A CTN contract is the interface specification for your scanner. It defines:

- What OBJECT fields are required/optional
- What STATE fields can be validated
- Which operations are supported for each field
- How ESP names map to collected data
- What behaviors are supported

### Contract Template

```rust
use esp_scanner_base::strategies::{
    BehaviorParameter, BehaviorType, CollectionMode, CollectionStrategy,
    CtnContract, ObjectFieldSpec, PerformanceHints, StateFieldSpec,
    SupportedBehavior,
};
use esp_scanner_base::types::common::{DataType, Operation};

pub fn create_your_ctn_contract() -> CtnContract {
    let mut contract = CtnContract::new("your_ctn_type".to_string());

    // 1. Define OBJECT requirements
    add_object_requirements(&mut contract);

    // 2. Define STATE requirements
    add_state_requirements(&mut contract);

    // 3. Configure field mappings
    configure_field_mappings(&mut contract);

    // 4. Set collection strategy
    set_collection_strategy(&mut contract);

    // 5. Add supported behaviors (optional)
    add_behaviors(&mut contract);

    contract
}
```

### 1. Object Requirements

Define what fields must be present in OBJECT blocks:

```rust
fn add_object_requirements(contract: &mut CtnContract) {
    // Required field - ESP validation fails if missing
    contract.object_requirements.add_required_field(ObjectFieldSpec {
        name: "resource_id".to_string(),
        data_type: DataType::String,
        description: "Unique identifier for the resource".to_string(),
        example_values: vec![
            "web-server-01".to_string(),
            "database-prod".to_string(),
        ],
        validation_notes: Some("Must be unique within definition".to_string()),
    });

    // Optional field - nice to have but not required
    contract.object_requirements.add_optional_field(ObjectFieldSpec {
        name: "description".to_string(),
        data_type: DataType::String,
        description: "Human-readable description".to_string(),
        example_values: vec!["Primary web server".to_string()],
        validation_notes: Some("For documentation purposes".to_string()),
    });
}
```

**ESP Usage:**

```esp
OBJECT web_server
    resource_id `web-server-01`      # Required
    description `Primary web server` # Optional
OBJECT_END
```

### 2. State Requirements

Define what fields can be validated in STATE blocks:

```rust
fn add_state_requirements(contract: &mut CtnContract) {
    // String field with multiple operations
    contract.state_requirements.add_optional_field(StateFieldSpec {
        name: "status".to_string(),
        data_type: DataType::String,
        allowed_operations: vec![
            Operation::Equals,
            Operation::NotEqual,
            Operation::Contains,
            Operation::PatternMatch,  // Regex support
        ],
        description: "Resource status".to_string(),
        example_values: vec![
            "running".to_string(),
            "stopped".to_string(),
            "degraded".to_string(),
        ],
        validation_notes: Some("Case-sensitive comparison".to_string()),
    });

    // Integer field with comparison operations
    contract.state_requirements.add_optional_field(StateFieldSpec {
        name: "cpu_usage".to_string(),
        data_type: DataType::Int,
        allowed_operations: vec![
            Operation::Equals,
            Operation::NotEqual,
            Operation::GreaterThan,
            Operation::LessThan,
            Operation::GreaterThanOrEqual,
            Operation::LessThanOrEqual,
        ],
        description: "CPU usage percentage".to_string(),
        example_values: vec!["50".to_string(), "80".to_string()],
        validation_notes: Some("Integer from 0-100".to_string()),
    });

    // Boolean field
    contract.state_requirements.add_optional_field(StateFieldSpec {
        name: "secure".to_string(),
        data_type: DataType::Boolean,
        allowed_operations: vec![Operation::Equals, Operation::NotEqual],
        description: "Whether resource is secured".to_string(),
        example_values: vec!["true".to_string(), "false".to_string()],
        validation_notes: None,
    });
}
```

**ESP Usage:**

```esp
STATE healthy_state
    status string = `running`
    cpu_usage int < 80
    secure boolean = true
STATE_END
```

### 3. Field Mappings

Map ESP field names to your internal data field names:

```rust
fn configure_field_mappings(contract: &mut CtnContract) {
    // COLLECTION MAPPINGS: OBJECT → Collector
    // Maps ESP object fields to collector parameters
    contract
        .field_mappings
        .collection_mappings
        .object_to_collection
        .insert("resource_id".to_string(), "internal_id".to_string());

    // Specify what fields collector MUST provide
    contract
        .field_mappings
        .collection_mappings
        .required_data_fields = vec![
        "status".to_string(),
        "cpu_usage".to_string(),
        "secure".to_string(),
    ];

    // Specify what fields are optional
    contract
        .field_mappings
        .collection_mappings
        .optional_data_fields = vec![
        "memory_usage".to_string(),
        "uptime".to_string(),
    ];

    // VALIDATION MAPPINGS: STATE → CollectedData
    // Maps ESP state field names to collected data field names
    contract
        .field_mappings
        .validation_mappings
        .state_to_data
        .insert("status".to_string(), "status".to_string());

    contract
        .field_mappings
        .validation_mappings
        .state_to_data
        .insert("cpu_usage".to_string(), "cpu_usage".to_string());

    contract
        .field_mappings
        .validation_mappings
        .state_to_data
        .insert("secure".to_string(), "secure".to_string());
}
```

**Why Field Mappings?**

- ESP uses user-friendly names (`status`)
- Internal systems may use different names (`resource_state`)
- Mappings decouple ESP definitions from implementation details

### 4. Collection Strategy

Specify how data should be collected:

```rust
fn set_collection_strategy(contract: &mut CtnContract) {
    contract.collection_strategy = CollectionStrategy {
        collector_type: "api".to_string(),  // Type identifier
        collection_mode: CollectionMode::Command,  // Or Metadata, Content
        required_capabilities: vec![
            "http_client".to_string(),
            "json_parser".to_string(),
        ],
        performance_hints: PerformanceHints {
            expected_collection_time_ms: Some(200),  // Average time
            memory_usage_mb: Some(5),                // Peak memory
            network_intensive: true,                 // Network required?
            cpu_intensive: false,                    // CPU intensive?
            requires_elevated_privileges: false,     // Root required?
        },
    };
}
```

### 5. Behaviors (Optional)

Define optional behaviors that modify collection:

```rust
fn add_behaviors(contract: &mut CtnContract) {
    // Flag behavior (on/off switch)
    contract.add_supported_behavior(SupportedBehavior {
        name: "include_metrics".to_string(),
        behavior_type: BehaviorType::Flag,
        parameters: vec![],
        description: "Include detailed performance metrics".to_string(),
        example: "BEHAVIOR include_metrics".to_string(),
    });

    // Parameter behavior (takes value)
    contract.add_supported_behavior(SupportedBehavior {
        name: "timeout".to_string(),
        behavior_type: BehaviorType::Parameter,
        parameters: vec![BehaviorParameter {
            name: "timeout".to_string(),
            data_type: DataType::Int,
            required: true,
            default_value: Some("30".to_string()),
            description: "API request timeout in seconds".to_string(),
        }],
        description: "Set API request timeout".to_string(),
        example: "BEHAVIOR timeout 60".to_string(),
    });

    // Multi-parameter behavior
    contract.add_supported_behavior(SupportedBehavior {
        name: "retry_policy".to_string(),
        behavior_type: BehaviorType::Parameter,
        parameters: vec![
            BehaviorParameter {
                name: "max_retries".to_string(),
                data_type: DataType::Int,
                required: true,
                default_value: Some("3".to_string()),
                description: "Maximum number of retry attempts".to_string(),
            },
            BehaviorParameter {
                name: "backoff".to_string(),
                data_type: DataType::Int,
                required: false,
                default_value: Some("1000".to_string()),
                description: "Backoff delay in milliseconds".to_string(),
            },
        ],
        description: "Configure retry behavior for failed requests".to_string(),
        example: "BEHAVIOR retry_policy max_retries 5 backoff 2000".to_string(),
    });
}
```

**ESP Usage:**

```esp
CTN your_ctn_type
    BEHAVIOR include_metrics
    BEHAVIOR timeout 60
    BEHAVIOR retry_policy max_retries 5 backoff 2000
    # ...
CTN_END
```

### Complete Contract Example

See `contracts/file_contracts.rs`, `contracts/rpm_contracts.rs`, or `contracts/systemd_contracts.rs` for full implementations.

---

## Implementing a Collector

A collector gathers data from the system. It implements the `CtnDataCollector` trait.

### Collector Template

```rust
use esp_scanner_base::execution::BehaviorHints;
use esp_scanner_base::strategies::{
    CollectedData, CollectionError, CtnContract, CtnDataCollector,
};
use esp_scanner_base::types::common::ResolvedValue;
use esp_scanner_base::types::execution_context::{
    ExecutableObject, ExecutableObjectElement,
};
use std::collections::HashMap;

pub struct YourCollector {
    id: String,
    // Add any state your collector needs
}

impl YourCollector {
    pub fn new() -> Self {
        Self {
            id: "your_collector".to_string(),
        }
    }

    /// Extract a field value from ExecutableObject
    fn extract_field(
        &self,
        object: &ExecutableObject,
        field_name: &str,
    ) -> Result<String, CollectionError> {
        for element in &object.elements {
            if let ExecutableObjectElement::Field { name, value, .. } = element {
                if name == field_name {
                    match value {
                        ResolvedValue::String(s) => return Ok(s.clone()),
                        _ => {
                            return Err(CollectionError::InvalidObjectConfiguration {
                                object_id: object.identifier.clone(),
                                reason: format!(
                                    "Field '{}' must be string, got {:?}",
                                    field_name, value
                                ),
                            })
                        }
                    }
                }
            }
        }

        Err(CollectionError::InvalidObjectConfiguration {
            object_id: object.identifier.clone(),
            reason: format!("Missing required field '{}'", field_name),
        })
    }

    /// Your collection logic
    fn collect_data(
        &self,
        resource_id: &str,
        object_id: &str,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        let mut data = CollectedData::new(
            object_id.to_string(),
            "your_ctn_type".to_string(),
            self.id.clone(),
        );

        // Check for behaviors
        let include_metrics = hints.has_flag("include_metrics");
        let timeout_secs = hints.get_parameter_as_int("timeout").unwrap_or(30);

        // Gather your data here
        // Example: API call, file read, command execution, etc.

        // Add collected fields
        data.add_field(
            "status".to_string(),
            ResolvedValue::String("running".to_string()),
        );

        data.add_field(
            "cpu_usage".to_string(),
            ResolvedValue::Integer(45),
        );

        data.add_field(
            "secure".to_string(),
            ResolvedValue::Boolean(true),
        );

        // Conditionally add metrics based on behavior
        if include_metrics {
            data.add_field(
                "memory_usage".to_string(),
                ResolvedValue::Integer(2048),
            );
        }

        Ok(data)
    }
}

impl CtnDataCollector for YourCollector {
    fn collect_for_ctn_with_hints(
        &self,
        object: &ExecutableObject,
        contract: &CtnContract,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        // 1. Validate behavior hints against contract
        contract.validate_behavior_hints(hints).map_err(|e| {
            CollectionError::CtnContractValidation {
                reason: e.to_string(),
            }
        })?;

        // 2. Extract required fields from object
        let resource_id = self.extract_field(object, "resource_id")?;

        // 3. Collect data
        self.collect_data(&resource_id, &object.identifier, hints)
    }

    fn supported_ctn_types(&self) -> Vec<String> {
        vec!["your_ctn_type".to_string()]
    }

    fn validate_ctn_compatibility(
        &self,
        contract: &CtnContract,
    ) -> Result<(), CollectionError> {
        if !self.supported_ctn_types().contains(&contract.ctn_type) {
            return Err(CollectionError::CtnContractValidation {
                reason: format!(
                    "CTN type '{}' not supported",
                    contract.ctn_type
                ),
            });
        }
        Ok(())
    }

    fn collector_id(&self) -> &str {
        &self.id
    }

    fn supports_batch_collection(&self) -> bool {
        false  // Set to true if you implement collect_batch
    }

    // Optional: Implement for performance optimization
    fn collect_batch(
        &self,
        objects: Vec<&ExecutableObject>,
        contract: &CtnContract,
    ) -> Result<HashMap<String, CollectedData>, CollectionError> {
        // Default implementation collects one by one
        // Override for batch operations (e.g., single API call for all objects)
        let mut results = HashMap::new();

        for object in objects {
            use esp_scanner_base::execution::extract_behavior_hints;
            let hints = extract_behavior_hints(object);
            let data = self.collect_for_ctn_with_hints(object, contract, &hints)?;
            results.insert(object.identifier.clone(), data);
        }

        Ok(results)
    }
}
```

### Collection Error Types

Use appropriate error types:

```rust
// File doesn't exist (triggers existence check failure)
Err(CollectionError::ObjectNotFound {
    object_id: object_id.to_string(),
})

// Permission denied (different from not found)
Err(CollectionError::AccessDenied {
    object_id: object_id.to_string(),
    reason: "Permission denied: ...".to_string(),
})

// General collection failure
Err(CollectionError::CollectionFailed {
    object_id: object_id.to_string(),
    reason: "Failed to fetch data: ...".to_string(),
})

// Invalid object configuration
Err(CollectionError::InvalidObjectConfiguration {
    object_id: object.identifier.clone(),
    reason: "Missing required field".to_string(),
})
```

### Working with Behavior Hints

```rust
fn collect_with_behaviors(
    &self,
    hints: &BehaviorHints,
) -> Result<CollectedData, CollectionError> {
    // Check if flag is set
    if hints.has_flag("verbose") {
        println!("Verbose mode enabled");
    }

    // Get parameter as integer
    let timeout = hints.get_parameter_as_int("timeout").unwrap_or(30);

    // Get parameter as string
    if let Some(endpoint) = hints.get_parameter_as_string("endpoint") {
        println!("Using endpoint: {}", endpoint);
    }

    // Get all parameters for a behavior
    if let Some(params) = hints.get_parameters("retry_policy") {
        let max_retries = params.get("max_retries")
            .and_then(|v| v.as_integer())
            .unwrap_or(3);
        let backoff = params.get("backoff")
            .and_then(|v| v.as_integer())
            .unwrap_or(1000);

        println!("Retry: {} attempts, {} ms backoff", max_retries, backoff);
    }

    // ... collection logic
}
```

### Collector Examples

- **File System**: `collectors/filesystem.rs`
- **Commands**: `collectors/command.rs`
- **Computed Values**: `collectors/computed_values.rs`

---

## Implementing an Executor

An executor validates collected data against STATE requirements. It implements the `CtnExecutor` trait.

### Executor Template

```rust
use esp_scanner_base::execution::{
    comparisons::string,  // For string operations
    evaluate_existence_check, evaluate_item_check, evaluate_state_operator,
};
use esp_scanner_base::strategies::{
    CollectedData, ComplianceStatus, CtnContract, CtnExecutionError,
    CtnExecutionResult, CtnExecutor, FieldValidationResult,
    StateValidationResult, TestPhase,
};
use esp_scanner_base::types::common::{Operation, ResolvedValue};
use esp_scanner_base::types::execution_context::ExecutableCriterion;
use std::collections::HashMap;

pub struct YourExecutor {
    contract: CtnContract,
}

impl YourExecutor {
    pub fn new(contract: CtnContract) -> Self {
        Self { contract }
    }

    /// Compare values based on operation and data type
    fn compare_values(
        &self,
        expected: &ResolvedValue,
        actual: &ResolvedValue,
        operation: Operation,
    ) -> bool {
        match (expected, actual, operation) {
            // String comparisons - USE string::compare for full operation support
            (ResolvedValue::String(exp), ResolvedValue::String(act), op) => {
                match string::compare(act, exp, op) {
                    Ok(result) => result,
                    Err(e) => {
                        eprintln!("String comparison error: {}", e);
                        false
                    }
                }
            }

            // Integer comparisons
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::Equals) => {
                act == exp
            }
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::NotEqual) => {
                act != exp
            }
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::GreaterThan) => {
                act > exp
            }
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::LessThan) => {
                act < exp
            }
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::GreaterThanOrEqual) => {
                act >= exp
            }
            (ResolvedValue::Integer(exp), ResolvedValue::Integer(act), Operation::LessThanOrEqual) => {
                act <= exp
            }

            // Boolean comparisons
            (ResolvedValue::Boolean(exp), ResolvedValue::Boolean(act), Operation::Equals) => {
                act == exp
            }
            (ResolvedValue::Boolean(exp), ResolvedValue::Boolean(act), Operation::NotEqual) => {
                act != exp
            }

            // Type mismatch or unsupported operation
            _ => false,
        }
    }
}

impl CtnExecutor for YourExecutor {
    fn execute_with_contract(
        &self,
        criterion: &ExecutableCriterion,
        collected_data: &HashMap<String, CollectedData>,
        _contract: &CtnContract,
    ) -> Result<CtnExecutionResult, CtnExecutionError> {
        let test_spec = &criterion.test;

        // ================================================================
        // PHASE 1: Existence Check
        // ================================================================
        let objects_expected = criterion.expected_object_count();
        let objects_found = collected_data.len();

        let existence_passed = evaluate_existence_check(
            test_spec.existence_check,
            objects_found,
            objects_expected,
        );

        if !existence_passed {
            return Ok(CtnExecutionResult::fail(
                criterion.criterion_type.clone(),
                format!(
                    "Existence check failed: expected {} objects, found {}",
                    objects_expected, objects_found
                ),
            ));
        }

        // ================================================================
        // PHASE 2: State Validation
        // ================================================================
        let mut state_results = Vec::new();
        let mut failure_messages = Vec::new();

        for (object_id, data) in collected_data {
            let mut all_field_results = Vec::new();

            // Validate each state for this object
            for state in &criterion.states {
                for field in &state.fields {
                    // Map ESP state field name to collected data field name
                    let data_field_name = self
                        .contract
                        .field_mappings
                        .validation_mappings
                        .state_to_data
                        .get(&field.name)
                        .cloned()
                        .unwrap_or_else(|| field.name.clone());

                    // Get collected value
                    let actual_value = match data.get_field(&data_field_name) {
                        Some(v) => v.clone(),
                        None => {
                            let msg = format!(
                                "Field '{}' not collected",
                                field.name
                            );
                            all_field_results.push(FieldValidationResult {
                                field_name: field.name.clone(),
                                expected_value: field.value.clone(),
                                actual_value: ResolvedValue::String("".to_string()),
                                operation: field.operation,
                                passed: false,
                                message: msg.clone(),
                            });
                            failure_messages.push(format!(
                                "Object '{}': {}",
                                object_id, msg
                            ));
                            continue;
                        }
                    };

                    // Perform comparison
                    let passed = self.compare_values(
                        &field.value,
                        &actual_value,
                        field.operation,
                    );

                    let msg = if passed {
                        format!("Field '{}' passed", field.name)
                    } else {
                        format!(
                            "Field '{}' failed: expected {:?}, got {:?}",
                            field.name, field.value, actual_value
                        )
                    };

                    if !passed {
                        failure_messages.push(format!(
                            "Object '{}': {}",
                            object_id, msg
                        ));
                    }

                    all_field_results.push(FieldValidationResult {
                        field_name: field.name.clone(),
                        expected_value: field.value.clone(),
                        actual_value,
                        operation: field.operation,
                        passed,
                        message: msg,
                    });
                }
            }

            // Combine field results using state operator (AND/OR)
            let state_bools: Vec<bool> = all_field_results
                .iter()
                .map(|r| r.passed)
                .collect();

            let combined = evaluate_state_operator(
                test_spec.state_operator,
                &state_bools,
            );

            state_results.push(StateValidationResult {
                object_id: object_id.clone(),
                state_results: all_field_results,
                combined_result: combined,
                state_operator: test_spec.state_operator,
                message: format!(
                    "Object '{}': {}",
                    object_id,
                    if combined { "passed" } else { "failed" }
                ),
            });
        }

        // ================================================================
        // PHASE 3: Item Check
        // ================================================================
        let objects_passing = state_results
            .iter()
            .filter(|r| r.combined_result)
            .count();

        let item_passed = evaluate_item_check(
            test_spec.item_check,
            objects_passing,
            state_results.len(),
        );

        // ================================================================
        // FINAL RESULT
        // ================================================================
        let final_status = if existence_passed && item_passed {
            ComplianceStatus::Pass
        } else {
            ComplianceStatus::Fail
        };

        let message = if final_status == ComplianceStatus::Pass {
            format!(
                "Validation passed: {} of {} objects compliant",
                objects_passing,
                state_results.len()
            )
        } else {
            format!(
                "Validation failed:\n  - {}",
                failure_messages.join("\n  - ")
            )
        };

        Ok(CtnExecutionResult {
            ctn_type: criterion.criterion_type.clone(),
            status: final_status,
            test_phase: TestPhase::Complete,
            existence_result: None,
            state_results,
            item_check_result: None,
            message,
            details: serde_json::json!({
                "failures": failure_messages,
                "objects_expected": objects_expected,
                "objects_found": objects_found,
                "objects_passing": objects_passing,
            }),
            execution_metadata: Default::default(),
        })
    }

    fn get_ctn_contract(&self) -> CtnContract {
        self.contract.clone()
    }

    fn ctn_type(&self) -> &str {
        "your_ctn_type"
    }

    fn validate_collected_data(
        &self,
        collected_data: &HashMap<String, CollectedData>,
        _contract: &CtnContract,
    ) -> Result<(), CtnExecutionError> {
        // Validate that required fields are present
        for data in collected_data.values() {
            for required_field in &self
                .contract
                .field_mappings
                .collection_mappings
                .required_data_fields
            {
                if !data.has_field(required_field) {
                    return Err(CtnExecutionError::MissingDataField {
                        field: required_field.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}
```

### String Operations

**CRITICAL**: Always use `string::compare()` for string operations - it handles ALL operations including pattern matching:

```rust
use esp_scanner_base::execution::comparisons::string;

// This handles: equals, not_equal, contains, not_contains, starts_with,
// ends_with, pattern_match, case_insensitive_equals, etc.
let passed = match string::compare(actual, expected, operation) {
    Ok(result) => result,
    Err(e) => {
        eprintln!("Comparison error: {}", e);
        false
    }
};
```

**Supported Operations:**

- `Operation::Equals` - Exact match
- `Operation::NotEqual` - Not equal
- `Operation::Contains` - Contains substring
- `Operation::NotContains` - Does not contain
- `Operation::StartsWith` - Starts with prefix
- `Operation::EndsWith` - Ends with suffix
- `Operation::PatternMatch` - Regex pattern matching
- `Operation::Matches` - Alias for pattern_match
- `Operation::CaseInsensitiveEquals` - Case-insensitive comparison

### Advanced: Record Validation

For structured data (JSON, nested records):

```rust
use esp_scanner_base::execution::record_validation::validate_record_checks;
use esp_scanner_base::types::common::RecordData;

// In your executor
for state in &criterion.states {
    if !state.record_checks.is_empty() {
        // Extract RecordData from collected data
        let record_data = match data.get_field("json_data") {
            Some(ResolvedValue::RecordData(rd)) => rd,
            _ => return Err(CtnExecutionError::DataValidationFailed {
                reason: "Expected RecordData".to_string(),
            }),
        };

        // Validate all record checks
        let validation_results = validate_record_checks(
            record_data,
            &state.record_checks,
        ).map_err(|e| CtnExecutionError::ExecutionFailed {
            ctn_type: criterion.criterion_type.clone(),
            reason: format!("Record validation failed: {}", e),
        })?;

        // Process results...
    }
}
```

### Executor Examples

- **File Metadata**: `executors/file_metadata.rs`
- **File Content**: `executors/file_content.rs`
- **JSON Records**: `executors/json_record.rs`
- **RPM Packages**: `executors/rpm_package.rs`
- **Systemd Services**: `executors/systemd_service.rs`

---

## Registering Your Scanner

Create a registry function that pairs collectors with executors:

```rust
// src/lib.rs
use esp_scanner_base::strategies::{CtnStrategyRegistry, StrategyError};

pub fn create_your_scanner_registry() -> Result<CtnStrategyRegistry, StrategyError> {
    let mut registry = CtnStrategyRegistry::new();

    // Create contract
    let contract = contracts::create_your_ctn_contract();

    // Register the strategy (collector + executor pair)
    registry.register_ctn_strategy(
        Box::new(collectors::YourCollector::new()),
        Box::new(executors::YourExecutor::new(contract)),
    )?;

    // Add more CTN types as needed...

    Ok(registry)
}
```

### Multiple CTN Types

```rust
pub fn create_comprehensive_registry() -> Result<CtnStrategyRegistry, StrategyError> {
    let mut registry = CtnStrategyRegistry::new();

    // Register multiple CTN types
    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileMetadataExecutor::new(
            contracts::create_file_metadata_contract()
        )),
    )?;

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileContentExecutor::new(
            contracts::create_file_content_contract()
        )),
    )?;

    // Shared collector for multiple executors
    let command_executor = commands::create_platform_command_executor();
    let command_collector = collectors::CommandCollector::new(
        "command-collector",
        command_executor,
    );

    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(executors::RpmPackageExecutor::new(
            contracts::create_rpm_package_contract()
        )),
    )?;

    registry.register_ctn_strategy(
        Box::new(command_collector),
        Box::new(executors::SystemdServiceExecutor::new(
            contracts::create_systemd_service_contract()
        )),
    )?;

    Ok(registry)
}
```

---

## Advanced Features

### 1. Batch Collection

Optimize performance by collecting multiple objects in one operation:

```rust
impl CtnDataCollector for YourCollector {
    fn supports_batch_collection(&self) -> bool {
        true  // Enable batch mode
    }

    fn collect_batch(
        &self,
        objects: Vec<&ExecutableObject>,
        contract: &CtnContract,
    ) -> Result<HashMap<String, CollectedData>, CollectionError> {
        // Extract all IDs
        let ids: Vec<String> = objects
            .iter()
            .filter_map(|obj| self.extract_field(obj, "resource_id").ok())
            .collect();

        // Single API call for all objects
        let all_data = self.fetch_bulk_data(&ids)?;

        // Map results back to objects
        let mut results = HashMap::new();
        for object in objects {
            let id = self.extract_field(object, "resource_id")?;
            if let Some(item_data) = all_data.get(&id) {
                results.insert(
                    object.identifier.clone(),
                    self.create_collected_data(object, item_data),
                );
            }
        }

        Ok(results)
    }
}
```

**Example**: RPM batch collection (`collectors/command.rs`)

- Single `rpm -qa` command lists ALL packages
- Individual queries matched against bulk result
- 100x faster than N individual queries

### 2. Command Execution

For command-based collectors, use `SystemCommandExecutor`:

```rust
use esp_scanner_base::strategies::SystemCommandExecutor;
use std::time::Duration;

// Create executor with whitelist
let mut executor = SystemCommandExecutor::with_timeout(Duration::from_secs(5));
executor.allow_commands(&["rpm", "systemctl", "sysctl"]);

// Execute command
let output = executor.execute(
    "rpm",
    &["-q", "openssl"],
    Some(Duration::from_secs(10)),
)?;

if output.exit_code == 0 {
    println!("stdout: {}", output.stdout);
} else {
    println!("stderr: {}", output.stderr);
}
```

**Security Features:**

- Whitelist-only execution
- Configurable timeouts
- Automatic cleanup
- No shell expansion

### 3. Filter Support

Objects can be filtered using FILTER blocks with STATE_REFs:

```esp
FILTER active_services
    STATE_REF running_check
    ACTION include
FILTER_END

OBJECT sshd_service
    FILTER_REF active_services
    service_name `sshd.service`
OBJECT_END
```

**Filters are evaluated automatically** by the execution engine before collection. Your collector receives only filtered objects.

### 4. SET Operations

Objects can be dynamically generated using SET operations:

```esp
SET security_packages
    rpm_package `openssl`
    rpm_package `audit`
    rpm_package `firewalld`
SET_END

OBJECT_REF security_packages
```

**SETs are expanded automatically** before execution. Your collector sees individual objects.

### 5. RUN Operations

Compute values at runtime:

```esp
RUN compute_threshold
    PARAM base_value int 100
    PARAM multiplier float 1.5
    OPERATION multiply base_value multiplier
    OUTPUT threshold
RUN_END

STATE cpu_check
    cpu_usage int < VAR threshold
STATE_END
```

**RUN operations execute automatically** during resolution. Your executor sees resolved values.

### 6. Record Checks (JSON Validation)

Validate nested JSON/structured data:

```esp
STATE user_validation
    RECORD json_data
        users[*].name string = `admin` AT_LEAST_ONE
        users[*].active boolean = true ALL
        config.timeout int > 30
    RECORD_END
STATE_END
```

**Use `validate_record_checks()`** in your executor - see `executors/json_record.rs`.

---

## Testing Your Implementation

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_creation() {
        let collector = YourCollector::new();
        assert_eq!(collector.collector_id(), "your_collector");
        assert!(collector.supported_ctn_types().contains(&"your_ctn_type".to_string()));
    }

    #[test]
    fn test_contract_validation() {
        let contract = create_your_ctn_contract();
        assert_eq!(contract.ctn_type, "your_ctn_type");
        assert!(!contract.object_requirements.required_fields.is_empty());
    }

    #[test]
    fn test_executor_comparison() {
        let contract = create_your_ctn_contract();
        let executor = YourExecutor::new(contract);

        let expected = ResolvedValue::String("running".to_string());
        let actual = ResolvedValue::String("running".to_string());

        assert!(executor.compare_values(&expected, &actual, Operation::Equals));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_full_scan() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry
    let registry = create_your_scanner_registry()?;

    // Create scanner
    let mut scanner = EspScanner::new(Arc::new(registry))?;

    // Scan ESP file
    let result = scanner.scan_file("test_definition.esp")?;

    // Verify results
    assert!(result.results.passed);
    assert_eq!(result.results.check.total_criteria, 1);

    Ok(())
}
```

### Test ESP File

```esp
DEFINITION test_your_scanner
MODULE esp 1.0.0

VAR resource_name string `test-resource`

OBJECT test_resource
    resource_id VAR resource_name
OBJECT_END

STATE healthy
    status string = `running`
    secure boolean = true
STATE_END

CTN your_ctn_type
    TEST all all
    STATE_REF healthy
    OBJECT_REF test_resource
CTN_END
```

---

## Complete Examples

### Example 1: File System Scanner

**Contract:** `contracts/file_contracts.rs`
**Collector:** `collectors/filesystem.rs`
**Executors:** `executors/file_metadata.rs`, `executors/file_content.rs`

**Features:**

- Metadata validation (permissions, owner, group)
- Content validation (string operations, pattern matching)
- Behavior support (recursive scan, include hidden files)
- Batch collection not needed (file I/O is parallel)

### Example 2: Command-Based Scanner

**Contracts:** `contracts/rpm_contracts.rs`, `contracts/systemd_contracts.rs`
**Collector:** `collectors/command.rs`
**Executors:** `executors/rpm_package.rs`, `executors/systemd_service.rs`

**Features:**

- Secure command execution with whitelist
- Behavior support (timeout configuration)
- Batch collection (single `rpm -qa` for all packages)
- Timeout handling per behavior

### Example 3: JSON Record Scanner

**Contract:** `contracts/json_contracts.rs`
**Collector:** `collectors/filesystem.rs` (reused!)
**Executor:** `executors/json_record.rs`

**Features:**

- JSON parsing and validation
- Record checks with field paths
- Wildcard path support (`users[*].name`)
- Entity checks (ALL, AT_LEAST_ONE, NONE, ONLY_ONE)

---

## Best Practices

### 1. Contract Design

✅ **DO:**

- Provide clear examples in field specs
- Document edge cases in validation notes
- Use meaningful field names
- Define behaviors for optional features

❌ **DON'T:**

- Add unnecessary required fields
- Use vague descriptions
- Expose internal implementation details in field names

### 2. Collector Implementation

✅ **DO:**

- Handle errors gracefully (ObjectNotFound vs AccessDenied)
- Validate behavior hints against contract
- Implement batch collection when beneficial
- Add comprehensive logging

❌ **DON'T:**

- Silently ignore errors
- Make slow API calls without timeout
- Collect data not specified in contract
- Use blocking operations without timeout

### 3. Executor Implementation

✅ **DO:**

- Use `string::compare()` for ALL string operations
- Use helper functions (`evaluate_existence_check`, etc.)
- Provide detailed failure messages
- Map fields using contract mappings

❌ **DON'T:**

- Implement your own string comparison logic
- Skip field mapping lookups
- Return generic error messages
- Assume field names match ESP names

### 4. Error Handling

✅ **DO:**

```rust
// Specific error types
Err(CollectionError::ObjectNotFound { object_id })
Err(CollectionError::AccessDenied { object_id, reason })

// Descriptive messages
format!("Failed to parse response: {}", error)
```

❌ **DON'T:**

```rust
// Generic errors
Err(CollectionError::CollectionFailed { reason: "error" })

// Vague messages
"Something went wrong"
```

### 5. Performance

✅ **DO:**

- Implement batch collection for expensive operations
- Cache results when appropriate
- Use behaviors to control collection depth
- Set realistic performance hints in contract

❌ **DON'T:**

- Make N identical API calls
- Load entire databases into memory
- Ignore timeout configurations
- Block on network calls without limit

---

## Checklist

Use this checklist when implementing a new scanner:

### Contract

- [ ] CTN type name is descriptive and unique
- [ ] Required object fields documented
- [ ] Optional object fields documented
- [ ] All state fields have allowed operations
- [ ] Field mappings configured
- [ ] Collection strategy specified
- [ ] Behaviors documented (if any)
- [ ] Performance hints realistic

### Collector

- [ ] Implements `CtnDataCollector` trait
- [ ] Validates behavior hints
- [ ] Handles all error cases
- [ ] Returns correct field names (per mapping)
- [ ] Provides all required fields
- [ ] Batch collection considered
- [ ] Tests written

### Executor

- [ ] Implements `CtnExecutor` trait
- [ ] Uses `string::compare()` for strings
- [ ] Uses helper functions for TEST evaluation
- [ ] Applies field mappings
- [ ] Three-phase validation (existence/state/item)
- [ ] Detailed failure messages
- [ ] Tests written

### Integration

- [ ] Contract, collector, executor registered
- [ ] End-to-end test with ESP file
- [ ] Documentation updated
- [ ] Example ESP file provided

---

## Troubleshooting

### "Field not found" errors

**Problem:** Executor can't find field in collected data

**Solution:** Check field mappings in contract

```rust
// ESP uses "permissions", collected data has "file_mode"
contract.field_mappings.validation_mappings.state_to_data
    .insert("permissions".to_string(), "file_mode".to_string());
```

### Pattern matching not working

**Problem:** Regex patterns fail validation

**Solution:** Use `string::compare()` - it handles pattern_match automatically

```rust
// ❌ Wrong
if actual.contains(expected) { ... }

// ✅ Correct
match string::compare(actual, expected, Operation::PatternMatch) {
    Ok(result) => result,
    Err(e) => { eprintln!("Pattern error: {}", e); false }
}
```

### Behavior hints ignored

**Problem:** BEHAVIOR directives don't affect collection

**Solution:** Validate hints and check for them

```rust
// Validate hints against contract
contract.validate_behavior_hints(hints)?;

// Check for flags
if hints.has_flag("verbose") { ... }

// Get parameters
let timeout = hints.get_parameter_as_int("timeout").unwrap_or(30);
```

### Batch collection not working

**Problem:** Batch collection returns empty results

**Solution:** Return true from `supports_batch_collection()`

```rust
fn supports_batch_collection(&self) -> bool {
    true  // Must return true!
}
```

---

## Additional Resources

### SDK Modules

- **`esp_scanner_base`** - Core framework, traits, execution engine
- **`esp_compiler`** - ESP parsing, AST, grammar definitions
- **`esp_scanner_sdk`** - Example implementations (reference)

### Key Traits

- **`CtnDataCollector`** - Data collection interface
- **`CtnExecutor`** - Validation interface
- **`CtnContract`** - Contract definition builder

### Helper Functions

- **`evaluate_existence_check()`** - TEST existence evaluation
- **`evaluate_item_check()`** - TEST item evaluation
- **`evaluate_state_operator()`** - AND/OR combination
- **`string::compare()`** - All string operations
- **`validate_record_checks()`** - JSON/record validation

### Example Implementations

All in `esp_scanner_sdk/src/`:

- File system scanners
- Command-based scanners
- JSON record scanners
- Computed value validators

---

## Summary

To create a new ESP scanner:

1. **Define Contract** - What you validate and how
2. **Implement Collector** - Gather system data
3. **Implement Executor** - Validate data against states
4. **Register Strategy** - Pair collector + executor
5. **Test** - Unit tests + integration tests

The esp_scanner_base handles everything else: parsing, resolution, execution orchestration, result generation, and error handling.

**Key Principles:**

- Contracts define interfaces
- Collectors gather data
- Executors validate data
- Registry connects components
- Framework handles orchestration

For questions or examples, refer to the reference implementations in `esp_scanner_sdk/src/`.
