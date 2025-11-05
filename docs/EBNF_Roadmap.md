# ESP Implementation Roadmap - Scanner Gaps (Architecture-Correct)

**Version:** 3.0
**Date:** 2024-11-04
**Corrected By:** Architecture Review

---

## Architecture Clarity

### ✅ Compiler: Syntax Validation ONLY (COMPLETE)

**What the compiler does:**

- ✅ Parse BEHAVIOR blocks → AST
- ✅ Parse FILTER specs → AST
- ✅ Parse SET operations → AST
- ✅ Parse pattern_match operations → AST
- ✅ Validate EBNF grammar compliance
- ✅ Build symbol tables
- ✅ Check type compatibility matrix

**What the compiler does NOT do:**

- ❌ Validate behavior names (scanner responsibility)
- ❌ Validate patterns are valid regex (scanner responsibility)
- ❌ Execute SET operations (scanner responsibility)
- ❌ Apply FILTER logic (scanner responsibility)

**Compiler Status:** ✅ **100% COMPLETE FOR ITS ROLE**

---

### ⚠️ Scanner Base: Execution Framework (NEEDS WORK)

**Role:** Provide infrastructure for strategy execution

**Components:**

- Contract system (declare what's supported)
- Execution helpers (SET expansion, filter evaluation)
- Type system (EVR strings, patterns)
- Comparison operations

---

### ⚠️ Scanner SDK: Concrete Implementations (NEEDS WORK)

**Role:** Implement actual CTN strategies

**Components:**

- Collectors (implement behaviors)
- Executors (validate collected data)
- Contracts (declare supported behaviors)

---

## Critical Gap Analysis

**Current Implementation: 82%**

### What Works ✅

- File metadata/content validation
- JSON validation
- RPM/Systemd/Sysctl basic checks (no SET, no complex BEHAVIOR)
- Type checking for simple operations

### What Doesn't Work ❌

- SET expansion during collection
- FILTER application
- BEHAVIOR-driven collection (recursive_scan, etc.)
- Pattern regex execution
- EVR version comparison

---

## Part 1: CRITICAL - Scanner Base Implementation

### 🔴 1.1 SET Operations Framework

**Status:** Parsed by compiler ✅, but scanner doesn't expand them ❌

**Problem:** When a CTN references a SET_REF, the scanner treats it as a single object instead of expanding it.

#### Scanner Base Changes Required

##### File: `esp_scanner_base/src/resolution/set_expansion.rs`

**Current:** Stub implementation
**Needs:** Complete SET expansion engine

```rust
//! SET expansion during object resolution

use crate::strategies::CtnStrategyRegistry;
use crate::types::execution_context::ExecutableObject;
use crate::types::resolution_context::ResolutionContext;
use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
pub enum SetExpansionError {
    #[error("SET '{set_id}' not found in resolution context")]
    SetNotFound { set_id: String },

    #[error("Circular SET reference detected: {cycle:?}")]
    CircularReference { cycle: Vec<String> },

    #[error("SET operand '{operand_id}' not found")]
    OperandNotFound { operand_id: String },

    #[error("Filter evaluation failed: {reason}")]
    FilterFailed { reason: String },
}

/// Expand a SET reference into concrete objects
pub fn expand_set_reference(
    set_id: &str,
    context: &ResolutionContext,
    registry: &CtnStrategyRegistry,
) -> Result<Vec<ExecutableObject>, SetExpansionError> {
    // Track visited SETs to detect cycles
    let mut visited = HashSet::new();
    expand_set_recursive(set_id, context, registry, &mut visited)
}

fn expand_set_recursive(
    set_id: &str,
    context: &ResolutionContext,
    registry: &CtnStrategyRegistry,
    visited: &mut HashSet<String>,
) -> Result<Vec<ExecutableObject>, SetExpansionError> {
    // Check for cycles
    if visited.contains(set_id) {
        return Err(SetExpansionError::CircularReference {
            cycle: visited.iter().cloned().collect(),
        });
    }
    visited.insert(set_id.to_string());

    // Get SET definition
    let set_def = context.get_set_definition(set_id)
        .ok_or_else(|| SetExpansionError::SetNotFound {
            set_id: set_id.to_string(),
        })?;

    // Expand each operand
    let mut operand_results: Vec<Vec<ExecutableObject>> = Vec::new();

    for operand in &set_def.operands {
        match operand {
            SetOperand::ObjectRef(obj_id) => {
                // Direct object reference
                let obj = context.get_object(obj_id)
                    .ok_or_else(|| SetExpansionError::OperandNotFound {
                        operand_id: obj_id.clone(),
                    })?;
                operand_results.push(vec![obj.clone()]);
            }

            SetOperand::SetRef(nested_set_id) => {
                // Recursive SET expansion
                let nested_objects = expand_set_recursive(
                    nested_set_id,
                    context,
                    registry,
                    visited,
                )?;
                operand_results.push(nested_objects);
            }

            SetOperand::InlineSet(inline_operands) => {
                // Expand inline set recursively
                let mut inline_objects = Vec::new();
                for inline_op in inline_operands {
                    let objs = expand_operand(inline_op, context, registry, visited)?;
                    inline_objects.extend(objs);
                }
                operand_results.push(inline_objects);
            }

            SetOperand::FilteredSet { set_id: filtered_id, filter } => {
                // Expand the SET first, then apply filter
                let unfiltered = expand_set_recursive(
                    filtered_id,
                    context,
                    registry,
                    visited,
                )?;

                let filtered = apply_filter_to_objects(
                    unfiltered,
                    filter,
                    context,
                    registry,
                )?;
                operand_results.push(filtered);
            }
        }
    }

    // Apply SET operation (union, intersection, complement)
    let result = apply_set_operation(&set_def.operation, operand_results)?;

    // Apply top-level filter if present
    if let Some(filter) = &set_def.filter {
        return apply_filter_to_objects(result, filter, context, registry);
    }

    visited.remove(set_id);
    Ok(result)
}

fn expand_operand(
    operand: &SetOperand,
    context: &ResolutionContext,
    registry: &CtnStrategyRegistry,
    visited: &mut HashSet<String>,
) -> Result<Vec<ExecutableObject>, SetExpansionError> {
    match operand {
        SetOperand::ObjectRef(obj_id) => {
            let obj = context.get_object(obj_id)
                .ok_or_else(|| SetExpansionError::OperandNotFound {
                    operand_id: obj_id.clone(),
                })?;
            Ok(vec![obj.clone()])
        }
        SetOperand::SetRef(set_id) => {
            expand_set_recursive(set_id, context, registry, visited)
        }
        SetOperand::InlineSet(operands) => {
            let mut objects = Vec::new();
            for op in operands {
                objects.extend(expand_operand(op, context, registry, visited)?);
            }
            Ok(objects)
        }
        SetOperand::FilteredSet { set_id, filter } => {
            let unfiltered = expand_set_recursive(set_id, context, registry, visited)?;
            apply_filter_to_objects(unfiltered, filter, context, registry)
        }
    }
}

/// Apply SET operation to operand results
fn apply_set_operation(
    operation: &SetOperationType,
    operand_results: Vec<Vec<ExecutableObject>>,
) -> Result<Vec<ExecutableObject>, SetExpansionError> {
    match operation {
        SetOperationType::Union => {
            // Union: combine all unique objects
            let mut result_map: HashMap<String, ExecutableObject> = HashMap::new();
            for operand_objs in operand_results {
                for obj in operand_objs {
                    result_map.insert(obj.identifier.clone(), obj);
                }
            }
            Ok(result_map.into_values().collect())
        }

        SetOperationType::Intersection => {
            // Intersection: only objects present in all operands
            if operand_results.is_empty() {
                return Ok(Vec::new());
            }

            let mut result: HashSet<String> = operand_results[0]
                .iter()
                .map(|obj| obj.identifier.clone())
                .collect();

            for operand_objs in &operand_results[1..] {
                let operand_ids: HashSet<String> = operand_objs
                    .iter()
                    .map(|obj| obj.identifier.clone())
                    .collect();

                result = result.intersection(&operand_ids).cloned().collect();
            }

            // Return objects that are in intersection
            let all_objects: HashMap<String, ExecutableObject> = operand_results
                .into_iter()
                .flatten()
                .map(|obj| (obj.identifier.clone(), obj))
                .collect();

            Ok(result
                .into_iter()
                .filter_map(|id| all_objects.get(&id).cloned())
                .collect())
        }

        SetOperationType::Complement => {
            // Complement: A - B (objects in first operand but not in second)
            if operand_results.len() != 2 {
                return Ok(Vec::new()); // Should be caught by semantic analysis
            }

            let set_a: HashSet<String> = operand_results[0]
                .iter()
                .map(|obj| obj.identifier.clone())
                .collect();

            let set_b: HashSet<String> = operand_results[1]
                .iter()
                .map(|obj| obj.identifier.clone())
                .collect();

            let difference: HashSet<String> = set_a.difference(&set_b).cloned().collect();

            Ok(operand_results[0]
                .iter()
                .filter(|obj| difference.contains(&obj.identifier))
                .cloned()
                .collect())
        }
    }
}

/// Apply FILTER to a list of objects
fn apply_filter_to_objects(
    objects: Vec<ExecutableObject>,
    filter: &FilterSpec,
    context: &ResolutionContext,
    registry: &CtnStrategyRegistry,
) -> Result<Vec<ExecutableObject>, SetExpansionError> {
    use crate::execution::filter_evaluation::FilterEvaluator;

    let evaluator = FilterEvaluator::new(registry.clone());
    evaluator.evaluate_filter(objects, filter, context)
        .map_err(|e| SetExpansionError::FilterFailed {
            reason: e.to_string(),
        })
}
```

**Estimated Effort:** 3-4 days

---

##### File: `esp_scanner_base/src/types/resolution_context.rs`

**Add SET lookup methods:**

```rust
impl ResolutionContext {
    /// Get SET definition by ID
    pub fn get_set_definition(&self, set_id: &str) -> Option<&SetDefinition> {
        self.sets.get(set_id)
    }

    /// Get object definition by ID
    pub fn get_object(&self, object_id: &str) -> Option<&ExecutableObject> {
        self.objects.get(object_id)
    }

    /// Get STATE definition by ID (for filter evaluation)
    pub fn get_state(&self, state_id: &str) -> Option<&StateDefinition> {
        self.states.get(state_id)
    }
}
```

**Estimated Effort:** 0.5 days

---

##### File: `esp_scanner_base/src/execution/filter_evaluation.rs`

**Current:** Stub
**Needs:** Complete filter evaluation

```rust
//! FILTER evaluation during SET expansion

use crate::execution::BehaviorHints;
use crate::strategies::{CollectedData, CtnStrategyRegistry};
use crate::types::execution_context::ExecutableObject;
use crate::types::resolution_context::ResolutionContext;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum FilterEvaluationError {
    #[error("State '{state_id}' not found for filter")]
    StateNotFound { state_id: String },

    #[error("Failed to collect data: {reason}")]
    CollectionFailed { reason: String },

    #[error("Field '{field}' not found in collected data")]
    FieldNotFound { field: String },

    #[error("Type mismatch in comparison: {details}")]
    TypeMismatch { details: String },
}

pub struct FilterEvaluator {
    registry: Arc<CtnStrategyRegistry>,
}

impl FilterEvaluator {
    pub fn new(registry: Arc<CtnStrategyRegistry>) -> Self {
        Self { registry }
    }

    /// Evaluate FILTER against objects
    pub fn evaluate_filter(
        &self,
        objects: Vec<ExecutableObject>,
        filter: &FilterSpec,
        context: &ResolutionContext,
    ) -> Result<Vec<ExecutableObject>, FilterEvaluationError> {
        let mut filtered = Vec::new();

        for object in objects {
            let matches = self.evaluate_object_against_filter(
                &object,
                filter,
                context,
            )?;

            // Apply filter action
            let should_include = match filter.action {
                FilterAction::Include => matches,
                FilterAction::Exclude => !matches,
            };

            if should_include {
                filtered.push(object);
            }
        }

        Ok(filtered)
    }

    fn evaluate_object_against_filter(
        &self,
        object: &ExecutableObject,
        filter: &FilterSpec,
        context: &ResolutionContext,
    ) -> Result<bool, FilterEvaluationError> {
        // For each state reference in filter, check if object matches
        for state_ref in &filter.state_refs {
            let state_def = context.get_state(&state_ref.state_id)
                .ok_or_else(|| FilterEvaluationError::StateNotFound {
                    state_id: state_ref.state_id.clone(),
                })?;

            // Collect data for this object
            let collected_data = self.collect_data_for_object(object)?;

            // Check if collected data matches state definition
            if !self.data_matches_state(&collected_data, state_def)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn collect_data_for_object(
        &self,
        object: &ExecutableObject,
    ) -> Result<CollectedData, FilterEvaluationError> {
        // Determine CTN type from object
        let ctn_type = self.determine_ctn_type(object)?;

        // Get collector and contract
        let collector = self.registry.get_collector(&ctn_type)
            .map_err(|e| FilterEvaluationError::CollectionFailed {
                reason: e.to_string(),
            })?;

        let contract = self.registry.get_ctn_contract(&ctn_type)
            .map_err(|e| FilterEvaluationError::CollectionFailed {
                reason: e.to_string(),
            })?;

        // Collect data
        collector.collect_for_ctn_with_hints(
            object,
            &contract,
            &BehaviorHints::empty(),
        ).map_err(|e| FilterEvaluationError::CollectionFailed {
            reason: e.to_string(),
        })
    }

    fn data_matches_state(
        &self,
        data: &CollectedData,
        state: &StateDefinition,
    ) -> Result<bool, FilterEvaluationError> {
        // Check each field in state definition
        for field in &state.fields {
            let actual = data.get_field(&field.name)
                .ok_or_else(|| FilterEvaluationError::FieldNotFound {
                    field: field.name.clone(),
                })?;

            // Compare values
            let matches = compare_values(&field.value, actual, field.operation)
                .map_err(|e| FilterEvaluationError::TypeMismatch {
                    details: e.to_string(),
                })?;

            if !matches {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn determine_ctn_type(
        &self,
        object: &ExecutableObject,
    ) -> Result<String, FilterEvaluationError> {
        // Look for module name in object elements
        for element in &object.elements {
            if let ExecutableObjectElement::Field { name, value, .. } = element {
                if name == "module" || name == "type" {
                    if let ResolvedValue::String(ctn_type) = value {
                        return Ok(ctn_type.clone());
                    }
                }
            }
        }

        Err(FilterEvaluationError::CollectionFailed {
            reason: "Could not determine CTN type from object".to_string(),
        })
    }
}

fn compare_values(
    expected: &ResolvedValue,
    actual: &ResolvedValue,
    operation: Operation,
) -> Result<bool, String> {
    use crate::execution::comparisons;

    match (expected, actual) {
        (ResolvedValue::String(exp), ResolvedValue::String(act)) => {
            comparisons::string::compare(act, exp, operation)
        }
        (ResolvedValue::Integer(exp), ResolvedValue::Integer(act)) => {
            comparisons::numeric::compare_i64(*act, *exp, operation)
        }
        (ResolvedValue::Boolean(exp), ResolvedValue::Boolean(act)) => {
            Ok(match operation {
                Operation::Equals => act == exp,
                Operation::NotEqual => act != exp,
                _ => return Err(format!("Invalid operation {:?} for booleans", operation)),
            })
        }
        _ => Err("Type mismatch in filter comparison".to_string()),
    }
}
```

**Estimated Effort:** 2-3 days

---

**🔴 Total SET Operations Effort: 6-8 days**

---

### 🔴 1.2 BEHAVIOR Framework (Scanner Base)

**Status:** Parser extracts ✅, but no validation framework in scanner

**Problem:** Collectors don't know what behaviors are supported, no runtime validation

#### Scanner Base Changes Required

##### File: `esp_scanner_base/src/strategies/contract.rs`

**Add behavior support to contracts:**

```rust
pub struct CtnContract {
    pub ctn_type: String,
    pub object_requirements: ObjectRequirements,
    pub state_requirements: StateRequirements,
    pub field_mappings: FieldMappings,
    pub collection_strategy: CollectionStrategy,

    // NEW: Declare supported behaviors
    pub supported_behaviors: Vec<BehaviorSpec>,
}

#[derive(Debug, Clone)]
pub struct BehaviorSpec {
    pub name: String,
    pub behavior_type: BehaviorType,
    pub description: String,
    pub example: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorType {
    Flag,        // behavior flag_name
    Parameter,   // behavior param_name=value
}

impl CtnContract {
    /// Check if a behavior is supported by this contract
    pub fn is_behavior_supported(&self, behavior_name: &str) -> bool {
        self.supported_behaviors
            .iter()
            .any(|spec| spec.name == behavior_name)
    }

    /// Get behavior specification
    pub fn get_behavior_spec(&self, behavior_name: &str) -> Option<&BehaviorSpec> {
        self.supported_behaviors
            .iter()
            .find(|spec| spec.name == behavior_name)
    }

    /// Validate behavior hints against contract
    pub fn validate_behaviors(
        &self,
        hints: &BehaviorHints,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Check flags
        for flag in hints.flags() {
            if !self.is_behavior_supported(flag) {
                errors.push(format!(
                    "Behavior '{}' not supported for CTN type '{}'",
                    flag,
                    self.ctn_type
                ));
            }
        }

        // Check parameters
        for param in hints.parameters() {
            if !self.is_behavior_supported(param) {
                errors.push(format!(
                    "Behavior parameter '{}' not supported for CTN type '{}'",
                    param,
                    self.ctn_type
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

**Estimated Effort:** 1 day

---

##### File: `esp_scanner_base/src/strategies/collector.rs`

**Add validation hooks:**

```rust
pub trait CtnDataCollector: Send + Sync {
    /// Collect data with hints (validates behaviors first)
    fn collect_for_ctn_with_hints(
        &self,
        object: &ExecutableObject,
        contract: &CtnContract,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        // Validate behaviors against contract
        if let Err(errors) = contract.validate_behaviors(hints) {
            return Err(CollectionError::UnsupportedBehaviors {
                ctn_type: contract.ctn_type.clone(),
                errors,
            });
        }

        // Delegate to actual collection
        self.collect_for_ctn_with_validated_hints(object, contract, hints)
    }

    /// Actual collection implementation (behaviors already validated)
    fn collect_for_ctn_with_validated_hints(
        &self,
        object: &ExecutableObject,
        contract: &CtnContract,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError>;

    // ... other methods ...
}
```

**New Error Type:**

```rust
#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    // ... existing variants ...

    #[error("Unsupported behaviors for CTN type '{ctn_type}': {}", errors.join(", "))]
    UnsupportedBehaviors {
        ctn_type: String,
        errors: Vec<String>,
    },
}
```

**Estimated Effort:** 1 day

---

**🔴 Total BEHAVIOR Framework Effort: 2 days**

---

## Part 2: CRITICAL - Scanner SDK Implementation

### 🔴 2.1 Contract Behavior Declarations

**Each contract must declare its supported behaviors**

#### File: `esp_scanner_sdk/src/contracts/file_contracts.rs`

```rust
pub fn create_file_metadata_contract() -> CtnContract {
    let mut contract = CtnContract::new("file_metadata".to_string());

    // ... existing object/state requirements ...

    // NEW: Declare supported behaviors
    contract.supported_behaviors = vec![
        BehaviorSpec {
            name: "recursive_scan".to_string(),
            behavior_type: BehaviorType::Flag,
            description: "Scan directories recursively".to_string(),
            example: "behavior recursive_scan".to_string(),
        },
        BehaviorSpec {
            name: "max_depth".to_string(),
            behavior_type: BehaviorType::Parameter,
            description: "Maximum recursion depth (default: 10)".to_string(),
            example: "behavior max_depth=5".to_string(),
        },
        BehaviorSpec {
            name: "follow_symlinks".to_string(),
            behavior_type: BehaviorType::Flag,
            description: "Follow symbolic links during scan".to_string(),
            example: "behavior follow_symlinks".to_string(),
        },
    ];

    contract
}

pub fn create_file_content_contract() -> CtnContract {
    let mut contract = CtnContract::new("file_content".to_string());

    // ... existing requirements ...

    contract.supported_behaviors = vec![
        BehaviorSpec {
            name: "recursive_scan".to_string(),
            behavior_type: BehaviorType::Flag,
            description: "Scan directories recursively".to_string(),
            example: "behavior recursive_scan".to_string(),
        },
        BehaviorSpec {
            name: "binary_mode".to_string(),
            behavior_type: BehaviorType::Flag,
            description: "Read files as binary data".to_string(),
            example: "behavior binary_mode".to_string(),
        },
        BehaviorSpec {
            name: "encoding".to_string(),
            behavior_type: BehaviorType::Parameter,
            description: "Text encoding (default: UTF-8)".to_string(),
            example: "behavior encoding=utf-16".to_string(),
        },
    ];

    contract
}
```

**Do the same for:**

- `rpm_contracts.rs` - Add `timeout`, `cache_results`
- `systemd_contracts.rs` - Add `timeout`
- `sysctl_contracts.rs` - Add `timeout`
- `selinux_contracts.rs` - (no behaviors needed currently)
- `json_contracts.rs` - (no behaviors needed currently)

**Estimated Effort:** 1 day

---

### 🔴 2.2 Behavior Implementation in Collectors

#### File: `esp_scanner_sdk/src/collectors/filesystem.rs`

**Implement declared behaviors:**

```rust
impl CtnDataCollector for FileSystemCollector {
    fn collect_for_ctn_with_validated_hints(
        &self,
        object: &ExecutableObject,
        contract: &CtnContract,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        let path = self.extract_path(object)?;

        // Handle recursive_scan behavior
        if hints.has_flag("recursive_scan") {
            let max_depth = hints.get_parameter_as_int("max_depth")
                .unwrap_or(10) as usize;

            return self.collect_recursive(
                &path,
                max_depth,
                &object.identifier,
                hints,
            );
        }

        // Handle follow_symlinks behavior
        let follow_symlinks = hints.has_flag("follow_symlinks");

        // Handle binary_mode for file_content
        if contract.ctn_type == "file_content" {
            if hints.has_flag("binary_mode") {
                return self.collect_binary_content(&path, &object.identifier);
            }

            // Handle custom encoding
            if let Some(encoding) = hints.get_parameter("encoding") {
                return self.collect_with_encoding(
                    &path,
                    encoding,
                    &object.identifier,
                );
            }
        }

        // Default behavior
        match contract.collection_strategy.collection_mode {
            CollectionMode::Metadata => {
                self.collect_metadata_with_options(
                    &path,
                    &object.identifier,
                    follow_symlinks,
                )
            }
            CollectionMode::Content => {
                self.collect_content(&path, &object.identifier)
            }
            _ => Err(CollectionError::UnsupportedCollectionMode {
                collector_id: self.id.clone(),
                mode: format!("{:?}", contract.collection_strategy.collection_mode),
            }),
        }
    }
}

// NEW METHODS TO IMPLEMENT:

impl FileSystemCollector {
    fn collect_recursive(
        &self,
        path: &str,
        max_depth: usize,
        object_id: &str,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        use walkdir::WalkDir;

        let follow_links = hints.has_flag("follow_symlinks");

        let walker = WalkDir::new(path)
            .max_depth(max_depth)
            .follow_links(follow_links);

        // Collect metadata for all files in directory tree
        let mut all_files = Vec::new();
        for entry in walker {
            let entry = entry.map_err(|e| CollectionError::CollectionFailed {
                object_id: object_id.to_string(),
                reason: format!("Directory walk failed: {}", e),
            })?;

            if entry.file_type().is_file() {
                all_files.push(entry.path().to_string_lossy().to_string());
            }
        }

        // Create aggregated metadata
        let mut data = CollectedData::new(
            object_id.to_string(),
            "file_metadata".to_string(),
            self.id.clone(),
        );

        data.add_field(
            "file_count".to_string(),
            ResolvedValue::Integer(all_files.len() as i64),
        );

        data.add_field(
            "files".to_string(),
            ResolvedValue::Collection(
                all_files.into_iter()
                    .map(ResolvedValue::String)
                    .collect()
            ),
        );

        Ok(data)
    }

    fn collect_metadata_with_options(
        &self,
        path: &str,
        object_id: &str,
        follow_symlinks: bool,
    ) -> Result<CollectedData, CollectionError> {
        let path_obj = if follow_symlinks {
            // Use fs::metadata (follows symlinks)
            Path::new(path)
        } else {
            // Use fs::symlink_metadata (doesn't follow)
            Path::new(path)
        };

        // Rest of metadata collection...
        self.collect_metadata(path, object_id)
    }

    fn collect_binary_content(
        &self,
        path: &str,
        object_id: &str,
    ) -> Result<CollectedData, CollectionError> {
        let mut data = CollectedData::new(
            object_id.to_string(),
            "file_content".to_string(),
            self.id.clone(),
        );

        let bytes = fs::read(path).map_err(|e| CollectionError::CollectionFailed {
            object_id: object_id.to_string(),
            reason: format!("Failed to read file: {}", e),
        })?;

        data.add_field(
            "file_content".to_string(),
            ResolvedValue::Binary(bytes),
        );

        Ok(data)
    }

    fn collect_with_encoding(
        &self,
        path: &str,
        encoding: &str,
        object_id: &str,
    ) -> Result<CollectedData, CollectionError> {
        use encoding_rs::Encoding;

        let bytes = fs::read(path).map_err(|e| CollectionError::CollectionFailed {
            object_id: object_id.to_string(),
            reason: format!("Failed to read file: {}", e),
        })?;

        let encoding_obj = Encoding::for_label(encoding.as_bytes())
            .ok_or_else(|| CollectionError::CollectionFailed {
                object_id: object_id.to_string(),
                reason: format!("Unknown encoding: {}", encoding),
            })?;

        let (decoded, _, had_errors) = encoding_obj.decode(&bytes);

        if had_errors {
            return Err(CollectionError::CollectionFailed {
                object_id: object_id.to_string(),
                reason: "Encoding errors encountered".to_string(),
            });
        }

        let mut data = CollectedData::new(
            object_id.to_string(),
            "file_content".to_string(),
            self.id.clone(),
        );

        data.add_field(
            "file_content".to_string(),
            ResolvedValue::String(decoded.to_string()),
        );

        Ok(data)
    }
}
```

**Add to `esp_scanner_sdk/Cargo.toml`:**

```toml
[dependencies]
walkdir = "2.4"
encoding_rs = "0.8"
```

**Estimated Effort:** 3 days

---

#### File: `esp_scanner_sdk/src/collectors/command.rs`

**Implement cache_results:**

```rust
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref COMMAND_CACHE: Mutex<HashMap<String, CollectedData>> = Mutex::new(HashMap::new());
}

impl CommandCollector {
    fn collect_rpm_package(
        &self,
        object: &ExecutableObject,
        hints: &BehaviorHints,
    ) -> Result<CollectedData, CollectionError> {
        let package_name = self.extract_field(object, "package_name")?;
        let cache_key = format!("rpm:{}", package_name);

        // Check cache if requested
        if hints.has_flag("cache_results") {
            if let Some(cached) = Self::get_cached(&cache_key) {
                return Ok(cached);
            }
        }

        // Perform collection
        let timeout = hints
            .get_parameter_as_int("timeout")
            .map(|t| std::time::Duration::from_secs(t as u64));

        let output = self
            .executor
            .execute("rpm", &["-q", &package_name], timeout)?;

        let mut data = CollectedData::new(
            object.identifier.clone(),
            "rpm_package".to_string(),
            self.id.clone(),
        );

        // ... populate data ...

        // Store in cache if requested
        if hints.has_flag("cache_results") {
            Self::store_cached(&cache_key, data.clone());
        }

        Ok(data)
    }

    fn get_cached(key: &str) -> Option<CollectedData> {
        COMMAND_CACHE.lock().unwrap().get(key).cloned()
    }

    fn store_cached(key: &str, data: CollectedData) {
        COMMAND_CACHE.lock().unwrap().insert(key.to_string(), data);
    }
}
```

**Add to `esp_scanner_sdk/Cargo.toml`:**

```toml
[dependencies]
lazy_static = "1.4"
```

**Estimated Effort:** 1 day

---

**🔴 Total BEHAVIOR SDK Implementation Effort: 5 days**

---

## Part 3: HIGH Priority - Comparison Operations

### 🟠 3.1 Pattern Matching Execution

**Status:** Parsed ✅, but pattern execution not implemented

#### Scanner Base Changes

##### File: `esp_scanner_base/src/execution/comparisons/pattern.rs` (NEW)

```rust
//! Pattern matching using regex

use super::ComparisonError;
use crate::types::common::Operation;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<HashMap<String, Regex>> = Mutex::new(HashMap::new());
}

pub fn pattern_match(text: &str, pattern: &str) -> Result<bool, ComparisonError> {
    let regex = get_or_compile_regex(pattern)?;
    Ok(regex.is_match(text))
}

fn get_or_compile_regex(pattern: &str) -> Result<Regex, ComparisonError> {
    let mut cache = REGEX_CACHE.lock().unwrap();

    if let Some(regex) = cache.get(pattern) {
        return Ok(regex.clone());
    }

    let regex = Regex::new(pattern).map_err(|e| ComparisonError::InvalidPattern {
        pattern: pattern.to_string(),
        reason: e.to_string(),
    })?;

    cache.insert(pattern.to_string(), regex.clone());
    Ok(regex)
}
```

**Update `string.rs`:**

```rust
pub fn compare(actual: &str, expected: &str, operation: Operation) -> Result<bool, ComparisonError> {
    match operation {
        // ... existing ...
        Operation::PatternMatch | Operation::Matches => {
            pattern::pattern_match(actual, expected)
        }
        // ...
    }
}
```

**Add to `esp_scanner_base/Cargo.toml`:**

```toml
[dependencies]
regex = "1.10"
lazy_static = "1.4"
```

**Estimated Effort:** 1 day

---

### 🟠 3.2 EVR Comparison Implementation

**Status:** Lexicographic comparison only

#### Scanner Base Changes

##### File: `esp_scanner_base/src/types/evr.rs` (NEW)

**Implement full RPM EVR algorithm (see detailed implementation in original roadmap)**

**Estimated Effort:** 3 days

##### File: `esp_scanner_base/src/execution/comparisons/evr.rs`

**Use EVR parser:**

```rust
pub fn compare_evr(actual: &str, expected: &str, operation: Operation) -> Result<bool, ComparisonError> {
    let actual_evr = EvrString::parse(actual)?;
    let expected_evr = EvrString::parse(expected)?;

    match operation {
        Operation::Equals => Ok(actual_evr == expected_evr),
        Operation::NotEqual => Ok(actual_evr != expected_evr),
        Operation::GreaterThan => Ok(actual_evr > expected_evr),
        Operation::LessThan => Ok(actual_evr < expected_evr),
        Operation::GreaterThanOrEqual => Ok(actual_evr >= expected_evr),
        Operation::LessThanOrEqual => Ok(actual_evr <= expected_evr),
        _ => Err(ComparisonError::UnsupportedOperation {
            operation: format!("{:?}", operation),
            data_type: "evr_string".to_string(),
        }),
    }
}
```

**Estimated Effort:** 1 day

---

**🟠 Total Comparison Operations Effort: 5 days**

---

## Summary Table (Corrected)

| Component | Files | Effort | Priority |
|-----------|-------|--------|----------|
| **SET Expansion** | 3 scanner_base files | 6-8 days | 🔴 CRITICAL |
| **BEHAVIOR Framework** | 2 scanner_base files | 2 days | 🔴 CRITICAL |
| **BEHAVIOR Contracts** | 7 SDK contract files | 1 day | 🔴 CRITICAL |
| **BEHAVIOR Collectors** | 2 SDK collector files | 4 days | 🔴 CRITICAL |
| **Pattern Execution** | 2 scanner_base files | 1 day | 🟠 HIGH |
| **EVR Comparison** | 2 scanner_base files | 4 days | 🟠 HIGH |
| **TOTAL CRITICAL** | - | **13-15 days** | - |
| **TOTAL HIGH** | - | **5 days** | - |
| **GRAND TOTAL** | - | **18-20 days** | - |

---

## Architecture-Correct Implementation Order

### Week 1: SET Operations (Critical)

**Days 1-3:** SET expansion engine (`set_expansion.rs`)
**Days 4-5:** Filter evaluation (`filter_evaluation.rs`)
**Day 6:** Resolution context updates
**Day 7:** Integration testing

### Week 2: BEHAVIOR Framework (Critical)

**Days 8-9:** Contract behavior support (scanner_base)
**Day 10:** Contract declarations (all SDK contracts)
**Days 11-14:** Collector implementations (filesystem + command)

### Week 3: Comparison Operations (High Priority)

**Day 15:** Pattern matching
**Days 16-19:** EVR comparison algorithm
**Day 20:** Integration testing

---

## Validation Tests Required

### SET Operations Tests

```rust
#[test]
fn test_set_union_basic() {
    // SET myfiles union
    //   OBJECT_REF file1
    //   OBJECT_REF file2
    // Verify 2 objects returned
}

#[test]
fn test_set_with_filter() {
    // SET filtered union
    //   OBJECT_REF files
    //   FILTER include STATE_REF readable
    // Verify only readable files returned
}

#[test]
fn test_nested_set_refs() {
    // SET all_configs union
    //   SET_REF system_configs
    //   SET_REF user_configs
    // Verify recursive expansion
}

#[test]
fn test_circular_set_detection() {
    // SET a references SET b
    // SET b references SET a
    // Verify error returned
}
```

### BEHAVIOR Tests

```rust
#[test]
fn test_unsupported_behavior_error() {
    // file_metadata with behavior cache_results
    // Verify error: cache_results not supported
}

#[test]
fn test_recursive_scan() {
    // file_metadata with behavior recursive_scan max_depth=3
    // Verify directory tree scanned
}

#[test]
fn test_binary_mode() {
    // file_content with behavior binary_mode
    // Verify binary data collected
}
```

### Pattern Tests

```rust
#[test]
fn test_pattern_match_basic() {
    assert!(pattern_match("test123", r"test\d+")?);
}

#[test]
fn test_invalid_pattern_error() {
    // Verify regex syntax error handled
}
```

### EVR Tests

```rust
#[test]
fn test_evr_epoch_precedence() {
    let v1 = EvrString::parse("1:1.0.0")?;
    let v2 = EvrString::parse("2.0.0")?;
    assert!(v1 > v2); // Epoch wins
}

#[test]
fn test_rpm_version_semantics() {
    let v1 = EvrString::parse("1.0.10")?;
    let v2 = EvrString::parse("1.0.9")?;
    assert!(v1 > v2); // Numeric not lexicographic
}
```
