# ESP EBNF Implementation Guide - Complete Missing Features

**Date:** November 4, 2025
**Status:** Implementation Ready
**Priority:** Critical Path Items First

---

## Executive Summary

Based on comprehensive code analysis, here are the CRITICAL gaps preventing full EBNF compliance:

### Implementation Priority Order

1. **🔴 CRITICAL - SET Expansion** (3-5 days)
2. **🔴 CRITICAL - FILTER Evaluation** (2-3 days)
3. **🟠 HIGH - BEHAVIOR Framework** (3-4 days)
4. **🟡 MEDIUM - Pattern Matching** (1 day)
5. **🟡 MEDIUM - EVR Comparison** (2-3 days)

**Total Estimated Effort:** 11-16 days

---

## Part 1: SET Expansion Implementation

### Current State Analysis

**File:** `esp_scanner_base/src/resolution/set_expansion.rs`

**Problems Found:**

1. Borrow checker issues in `expand_sets_in_resolution_context()`
2. Missing integration with execution phase
3. Incomplete filter application during expansion
4. Circular dependency detection works but not integrated

**What Works:**

- Helper functions for operand expansion
- Set operation application (union, intersection, complement)
- Validation logic structure

### Required Changes

#### 1.1 Fix set_expansion.rs Borrow Checker Issues

**Problem:** Lines 44-47 try to borrow context mutably while reading from it

```rust
// CURRENT BROKEN CODE (Lines 44-47):
for tree in &mut context.criteria_root.trees {
    total_expanded += expand_set_refs_in_criteria_tree(
        tree,
        context, // ERROR: Cannot borrow context again
    )?;
}
```

**Solution:** Clone the data needed for read-only access

```rust
// FIXED VERSION:
let resolved_sets = context.resolved_sets.clone();
let resolved_global_objects = context.resolved_global_objects.clone();

let mut total_expanded = 0;
for tree in &mut context.criteria_root.trees {
    total_expanded += expand_set_refs_in_criteria_tree_helper(
        tree,
        &resolved_sets,
        &resolved_global_objects,
    )?;
}
```

#### 1.2 Complete SET_REF Expansion Logic

The expansion logic is partially there but needs these additions:

```rust
// Add to set_expansion.rs

/// Main entry point - called from ResolutionEngine
pub fn expand_all_sets(context: &mut ResolutionContext) -> Result<(), ResolutionError> {
    // 1. Validate no circular dependencies
    validate_set_expansions(context)?;

    // 2. Expand SET_REF in criteria trees
    expand_sets_in_resolution_context(context)?;

    Ok(())
}

/// Expand SET_REF in a single criterion declaration
fn expand_set_ref_in_declaration(
    declaration: &mut CriterionDeclaration,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
    resolved_objects: &HashMap<String, ResolvedObject>,
) -> Result<bool, ResolutionError> {
    let mut was_expanded = false;
    let mut new_object_refs = Vec::new();

    // Check local object for SET_REF
    if let Some(local_obj) = &declaration.local_object {
        if let Some(set_id) = extract_set_ref_from_object(local_obj) {
            // Expand SET to object references
            let expanded_refs = expand_set_to_objects(
                &set_id,
                resolved_sets,
                resolved_objects,
            )?;

            new_object_refs.extend(expanded_refs);
            was_expanded = true;

            // Clear local object - it was just a SET_REF container
            declaration.local_object = None;
        }
    }

    // Check object_refs for SET_REF
    let mut refs_to_remove = HashSet::new();
    for obj_ref in &declaration.object_refs {
        if let Some(resolved_obj) = resolved_objects.get(&obj_ref.object_id) {
            if contains_set_ref(resolved_obj) {
                let set_id = extract_set_ref_from_resolved_object(resolved_obj)?;
                let expanded = expand_set_to_objects(&set_id, resolved_sets, resolved_objects)?;
                new_object_refs.extend(expanded);
                was_expanded = true;
                refs_to_remove.insert(obj_ref.object_id.clone());
            }
        }
    }

    // Update declaration
    if was_expanded {
        declaration.object_refs.retain(|r| !refs_to_remove.contains(&r.object_id));
        declaration.object_refs.extend(new_object_refs);

        // Deduplicate
        let mut seen = HashSet::new();
        declaration.object_refs.retain(|r| seen.insert(r.object_id.clone()));
    }

    Ok(was_expanded)
}

/// Extract SET_REF ID from object if present
fn extract_set_ref_from_object(obj: &ObjectDeclaration) -> Option<String> {
    for element in &obj.elements {
        if let ObjectElement::SetRef { set_id, .. } = element {
            return Some(set_id.clone());
        }
    }
    None
}

/// Check if resolved object contains SET_REF
fn contains_set_ref(obj: &ResolvedObject) -> bool {
    use crate::types::object::ResolvedObjectElement;
    obj.resolved_elements.iter().any(|elem| {
        matches!(elem, ResolvedObjectElement::SetRef { .. })
    })
}

/// Expand a SET to concrete object references
fn expand_set_to_objects(
    set_id: &str,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
    resolved_objects: &HashMap<String, ResolvedObject>,
) -> Result<Vec<ObjectRef>, ResolutionError> {
    let resolved_set = resolved_sets.get(set_id)
        .ok_or_else(|| ResolutionError::UndefinedSet {
            name: set_id.to_string(),
            context: "SET expansion".to_string(),
        })?;

    let mut object_refs = Vec::new();

    // Expand each operand
    for operand in &resolved_set.operands {
        match operand {
            ResolvedSetOperand::ObjectRef(obj_id) => {
                object_refs.push(ObjectRef {
                    object_id: obj_id.clone(),
                    span: None,
                });
            }
            ResolvedSetOperand::InlineObject { identifier } => {
                object_refs.push(ObjectRef {
                    object_id: identifier.clone(),
                    span: None,
                });
            }
            ResolvedSetOperand::SetRef(nested_set_id) => {
                // Recursive expansion
                let nested = expand_set_to_objects(
                    nested_set_id,
                    resolved_sets,
                    resolved_objects,
                )?;
                object_refs.extend(nested);
            }
            ResolvedSetOperand::FilteredObjectRef { object_id, filter: _ } => {
                // Add object - filter will be applied during execution
                object_refs.push(ObjectRef {
                    object_id: object_id.clone(),
                    span: None,
                });
            }
        }
    }

    // Note: Top-level filter is stored in resolved_set.filter
    // It will be applied during execution, not here

    Ok(object_refs)
}
```

#### 1.3 Integration with ResolutionEngine

**File:** `esp_scanner_base/src/resolution/engine.rs`

Add SET expansion to the resolution pipeline:

```rust
impl ResolutionEngine {
    pub fn resolve_context(
        &mut self,
        context: &mut ResolutionContext,
    ) -> Result<ExecutionContext, ResolutionError> {
        // Existing phases...
        // 1. Resolve DAG
        // 2. Resolve variables
        // 3. Resolve states
        // 4. Resolve objects
        // 5. Resolve sets

        // NEW PHASE 6: Expand SET_REF in criteria
        log_info!("Phase 6: Expanding SET references in criteria");
        set_expansion::expand_all_sets(context)?;

        // Continue with remaining phases...
        // 7. Create execution context

        Ok(execution_context)
    }
}
```

---

## Part 2: FILTER Evaluation Implementation

### Current State Analysis

**File:** `esp_scanner_base/src/execution/filter_evaluation.rs`

**Problems:**

1. Stub implementation - basic structure only
2. Not integrated with collection phase
3. State evaluation incomplete
4. No filter application in ExecutionEngine

### Required Implementation

#### 2.1 Complete FilterEvaluator

```rust
// COMPLETE IMPLEMENTATION

use crate::strategies::{CollectedData, CtnExecutionError};
use crate::types::execution_context::ExecutionContext;
use crate::types::filter::{FilterResult, ResolvedFilterSpec};
use crate::types::{ExecutableObject, ExecutableState};
use crate::execution::comparisons;
use std::collections::HashMap;

pub struct FilterEvaluator;

impl FilterEvaluator {
    /// Evaluate filter against collected data
    /// Returns true if object should be INCLUDED
    pub fn evaluate_filter(
        filter: &ResolvedFilterSpec,
        collected_data: &CollectedData,
        context: &ExecutionContext,
    ) -> Result<bool, FilterEvaluationError> {
        // All states in filter must be satisfied (AND logic)
        for state_ref in &filter.state_refs {
            let state = context.global_states.get(state_ref)
                .ok_or_else(|| FilterEvaluationError::StateNotFound {
                    state_id: state_ref.clone(),
                })?;

            let satisfied = Self::evaluate_state_against_data(state, collected_data)?;

            if !satisfied {
                // Apply filter action
                return Ok(filter.action == FilterAction::Exclude);
            }
        }

        // All states satisfied
        Ok(filter.action == FilterAction::Include)
    }

    /// Evaluate a single state against collected data
    fn evaluate_state_against_data(
        state: &ExecutableState,
        data: &CollectedData,
    ) -> Result<bool, FilterEvaluationError> {
        // Check each field in state
        for field in &state.fields {
            // Get actual value from collected data
            let actual_value = data.get_field(&field.name)
                .ok_or_else(|| FilterEvaluationError::FieldNotFound {
                    field_name: field.name.clone(),
                    object_id: data.object_id.clone(),
                })?;

            // Compare using existing comparison engine
            let passed = comparisons::compare_values(
                &field.value,
                actual_value,
                field.operation,
            ).map_err(|e| FilterEvaluationError::ComparisonFailed {
                reason: e.to_string(),
            })?;

            if !passed {
                return Ok(false); // One field fails = state fails
            }
        }

        Ok(true) // All fields passed
    }

    /// Apply object filters to collected data map
    /// Returns only objects that pass all filters
    pub fn apply_object_filters(
        collected: HashMap<String, (ExecutableObject, CollectedData)>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, CollectedData>, FilterEvaluationError> {
        let mut filtered = HashMap::new();

        for (object_id, (object, data)) in collected {
            let should_include = Self::should_include_object(&object, &data, context)?;

            if should_include {
                filtered.insert(object_id, data);
            }
        }

        Ok(filtered)
    }

    /// Check if object passes all its filters
    fn should_include_object(
        object: &ExecutableObject,
        data: &CollectedData,
        context: &ExecutionContext,
    ) -> Result<bool, FilterEvaluationError> {
        let filters = object.get_filters();

        if filters.is_empty() {
            return Ok(true); // No filters = include
        }

        // ALL filters must pass (AND logic)
        for filter in filters {
            let include = Self::evaluate_filter(filter, data, context)?;
            if !include {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Apply SET-level filter to objects
    pub fn apply_set_filter(
        objects: Vec<ExecutableObject>,
        filter: &ResolvedFilterSpec,
        collected_data: &HashMap<String, CollectedData>,
        context: &ExecutionContext,
    ) -> Result<Vec<ExecutableObject>, FilterEvaluationError> {
        let mut filtered = Vec::new();

        for object in objects {
            // Get collected data for this object
            if let Some(data) = collected_data.get(&object.identifier) {
                let include = Self::evaluate_filter(filter, data, context)?;
                if include {
                    filtered.push(object);
                }
            }
        }

        Ok(filtered)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FilterEvaluationError {
    #[error("Filter references undefined state: {state_id}")]
    StateNotFound { state_id: String },

    #[error("Field '{field_name}' not found in collected data for object '{object_id}'")]
    FieldNotFound {
        field_name: String,
        object_id: String,
    },

    #[error("Comparison failed: {reason}")]
    ComparisonFailed { reason: String },

    #[error("Invalid filter: {reason}")]
    InvalidFilter { reason: String },
}
```

#### 2.2 Integration with ExecutionEngine

**File:** `esp_scanner_base/src/execution/engine.rs`

Add filter evaluation to collection phase:

```rust
impl ExecutionEngine {
    fn collect_data_for_criterion(
        &mut self,
        criterion: &ExecutableCriterion,
    ) -> Result<HashMap<String, CollectedData>, ExecutionError> {
        // Existing collection logic...
        let mut collected = self.collect_raw_data(criterion)?;

        // NEW: Apply object filters
        let collected_with_objects: HashMap<String, (ExecutableObject, CollectedData)> =
            collected.into_iter()
                .filter_map(|(obj_id, data)| {
                    criterion.objects.iter()
                        .find(|o| o.identifier == obj_id)
                        .map(|obj| (obj_id, (obj.clone(), data)))
                })
                .collect();

        collected = FilterEvaluator::apply_object_filters(
            collected_with_objects,
            &self.context,
        ).map_err(|e| ExecutionError::FilterEvaluationFailed {
            reason: e.to_string(),
        })?;

        Ok(collected)
    }
}
```

---

## Part 3: BEHAVIOR Framework Implementation

### Current State

**What Works:**

- `BehaviorHints` extraction exists
- Collectors check for hints
- Basic behaviors like `timeout` are supported

**What's Missing:**

- Contract-level behavior declarations
- Validation of behavior support
- Documentation of available behaviors

### Required Changes

#### 3.1 Extend CtnContract with Behavior Support

**File:** `esp_scanner_base/src/strategies/ctn_contract.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtnContract {
    // ... existing fields ...

    /// Behaviors supported by this CTN type
    pub supported_behaviors: Vec<BehaviorSpec>,
}

/// Specification of a supported behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSpec {
    /// Behavior name (e.g., "recursive_scan", "timeout")
    pub name: String,

    /// Behavior description
    pub description: String,

    /// Parameters required/supported
    pub parameters: Vec<BehaviorParameter>,

    /// Whether this behavior is required or optional
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorParameter {
    pub name: String,
    pub param_type: String, // "int", "string", "boolean"
    pub required: bool,
    pub default_value: Option<String>,
    pub description: String,
}

impl CtnContract {
    /// Check if a behavior is supported
    pub fn supports_behavior(&self, behavior_name: &str) -> bool {
        self.supported_behaviors.iter()
            .any(|b| b.name == behavior_name)
    }

    /// Get behavior specification
    pub fn get_behavior(&self, behavior_name: &str) -> Option<&BehaviorSpec> {
        self.supported_behaviors.iter()
            .find(|b| b.name == behavior_name)
    }

    /// Validate behavior hints against contract
    pub fn validate_behaviors(
        &self,
        hints: &BehaviorHints,
    ) -> Result<(), String> {
        // Check each hint against supported behaviors
        for hint_name in hints.get_all_flags() {
            if !self.supports_behavior(&hint_name) {
                return Err(format!(
                    "Behavior '{}' not supported by CTN type '{}'",
                    hint_name, self.ctn_type
                ));
            }
        }

        // Check required behaviors are present
        for behavior in &self.supported_behaviors {
            if behavior.required && !hints.has_flag(&behavior.name) {
                return Err(format!(
                    "Required behavior '{}' not specified for CTN type '{}'",
                    behavior.name, self.ctn_type
                ));
            }
        }

        Ok(())
    }
}
```

#### 3.2 Update Contract Definitions

**File:** `esp_scanner_sdk/src/contracts/file_contracts.rs`

```rust
pub fn create_file_metadata_contract() -> CtnContract {
    let mut contract = CtnContract::new("file_metadata".to_string());

    // ... existing field definitions ...

    // Add behavior support
    contract.supported_behaviors = vec![
        BehaviorSpec {
            name: "recursive_scan".to_string(),
            description: "Recursively scan directory tree".to_string(),
            parameters: vec![
                BehaviorParameter {
                    name: "max_depth".to_string(),
                    param_type: "int".to_string(),
                    required: false,
                    default_value: Some("10".to_string()),
                    description: "Maximum directory depth".to_string(),
                },
            ],
            required: false,
        },
        BehaviorSpec {
            name: "follow_symlinks".to_string(),
            description: "Follow symbolic links during scan".to_string(),
            parameters: vec![],
            required: false,
        },
    ];

    contract
}

pub fn create_file_content_contract() -> CtnContract {
    let mut contract = CtnContract::new("file_content".to_string());

    // ... existing field definitions ...

    contract.supported_behaviors = vec![
        BehaviorSpec {
            name: "binary_mode".to_string(),
            description: "Read file in binary mode".to_string(),
            parameters: vec![],
            required: false,
        },
        BehaviorSpec {
            name: "encoding".to_string(),
            description: "Specify file encoding".to_string(),
            parameters: vec![
                BehaviorParameter {
                    name: "charset".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    default_value: Some("UTF-8".to_string()),
                    description: "Character encoding".to_string(),
                },
            ],
            required: false,
        },
    ];

    contract
}
```

#### 3.3 Update Command-Based Contracts

**File:** `esp_scanner_sdk/src/contracts/rpm_contracts.rs`

```rust
pub fn create_rpm_package_contract() -> CtnContract {
    let mut contract = CtnContract::new("rpm_package".to_string());

    // ... existing field definitions ...

    contract.supported_behaviors = vec![
        BehaviorSpec {
            name: "timeout".to_string(),
            description: "Command execution timeout in seconds".to_string(),
            parameters: vec![
                BehaviorParameter {
                    name: "seconds".to_string(),
                    param_type: "int".to_string(),
                    required: false,
                    default_value: Some("5".to_string()),
                    description: "Timeout duration".to_string(),
                },
            ],
            required: false,
        },
        BehaviorSpec {
            name: "cache_results".to_string(),
            description: "Cache RPM query results for reuse".to_string(),
            parameters: vec![
                BehaviorParameter {
                    name: "ttl".to_string(),
                    param_type: "int".to_string(),
                    required: false,
                    default_value: Some("300".to_string()),
                    description: "Cache TTL in seconds".to_string(),
                },
            ],
            required: false,
        },
    ];

    contract
}
```

---

## Part 4: Pattern Matching Implementation

### Required Files

#### 4.1 New Pattern Module

**File:** `esp_scanner_base/src/execution/comparisons/pattern.rs` (NEW)

```rust
//! Pattern matching using regex

use super::ComparisonError;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<HashMap<String, Regex>> = Mutex::new(HashMap::new());
}

/// Match text against regex pattern
pub fn pattern_match(text: &str, pattern: &str) -> Result<bool, ComparisonError> {
    let regex = get_or_compile_regex(pattern)?;
    Ok(regex.is_match(text))
}

/// Get regex from cache or compile and cache it
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

/// Clear regex cache (useful for testing)
pub fn clear_cache() {
    let mut cache = REGEX_CACHE.lock().unwrap();
    cache.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pattern_match() {
        assert!(pattern_match("test123", r"test\d+").unwrap());
        assert!(!pattern_match("test", r"test\d+").unwrap());
    }

    #[test]
    fn test_pattern_cache() {
        // First match compiles and caches
        assert!(pattern_match("hello", r"h.*o").unwrap());

        // Second match uses cache
        assert!(pattern_match("halo", r"h.*o").unwrap());
    }

    #[test]
    fn test_invalid_pattern() {
        let result = pattern_match("test", r"[invalid");
        assert!(result.is_err());
    }
}
```

#### 4.2 Integration with Comparisons

**File:** `esp_scanner_base/src/execution/comparisons/string.rs`

```rust
pub mod pattern; // Add module

pub fn compare(
    actual: &str,
    expected: &str,
    operation: Operation,
) -> Result<bool, ComparisonError> {
    match operation {
        // ... existing operations ...

        Operation::PatternMatch | Operation::Matches => {
            pattern::pattern_match(actual, expected)
        }

        // ... rest ...
    }
}
```

#### 4.3 Add Dependencies

**File:** `esp_scanner_base/Cargo.toml`

```toml
[dependencies]
# ... existing ...
regex = "1.10"
lazy_static = "1.4"
```

---

## Part 5: EVR Comparison Implementation

### Required Files

#### 5.1 EVR Type Module

**File:** `esp_scanner_base/src/types/evr.rs` (NEW)

```rust
//! RPM EVR (Epoch-Version-Release) string parsing and comparison
//!
//! Implements the RPM version comparison algorithm used by yum/dnf

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvrString {
    pub epoch: Option<u32>,
    pub version: String,
    pub release: Option<String>,
}

impl EvrString {
    /// Parse EVR string in format: [epoch:]version[-release]
    ///
    /// Examples:
    /// - "1.0.0" -> epoch=None, version="1.0.0", release=None
    /// - "2:1.0.0" -> epoch=Some(2), version="1.0.0", release=None
    /// - "1.0.0-27.el9" -> epoch=None, version="1.0.0", release=Some("27.el9")
    /// - "2:1.0.0-27.el9" -> epoch=Some(2), version="1.0.0", release=Some("27.el9")
    pub fn parse(s: &str) -> Result<Self, EvrParseError> {
        let s = s.trim();

        if s.is_empty() {
            return Err(EvrParseError::EmptyString);
        }

        // Extract epoch if present (before first ':')
        let (epoch, remainder) = if let Some(colon_pos) = s.find(':') {
            let epoch_str = &s[..colon_pos];
            let epoch = epoch_str.parse::<u32>()
                .map_err(|_| EvrParseError::InvalidEpoch(epoch_str.to_string()))?;
            (Some(epoch), &s[colon_pos + 1..])
        } else {
            (None, s)
        };

        // Split on last '-' to separate version from release
        let (version, release) = if let Some(dash_pos) = remainder.rfind('-') {
            let version = remainder[..dash_pos].to_string();
            let release = remainder[dash_pos + 1..].to_string();
            (version, Some(release))
        } else {
            (remainder.to_string(), None)
        };

        Ok(Self {
            epoch,
            version,
            release,
        })
    }

    /// Compare two EVR strings using RPM semantics
    fn compare_segments(a: &str, b: &str) -> Ordering {
        let mut a_chars = a.chars().peekable();
        let mut b_chars = b.chars().peekable();

        loop {
            // Skip separators
            while a_chars.peek().map_or(false, |c| !c.is_alphanumeric()) {
                a_chars.next();
            }
            while b_chars.peek().map_or(false, |c| !c.is_alphanumeric()) {
                b_chars.next();
            }

            // Collect alpha segment
            let a_alpha: String = a_chars.clone()
                .take_while(|c| c.is_alphabetic())
                .collect();
            let b_alpha: String = b_chars.clone()
                .take_while(|c| c.is_alphabetic())
                .collect();

            if !a_alpha.is_empty() || !b_alpha.is_empty() {
                // Advance iterators
                for _ in 0..a_alpha.len() {
                    a_chars.next();
                }
                for _ in 0..b_alpha.len() {
                    b_chars.next();
                }

                // Lexicographic comparison of alpha
                match a_alpha.cmp(&b_alpha) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }

            // Collect numeric segment
            let a_num_str: String = a_chars.clone()
                .take_while(|c| c.is_numeric())
                .collect();
            let b_num_str: String = b_chars.clone()
                .take_while(|c| c.is_numeric())
                .collect();

            if !a_num_str.is_empty() || !b_num_str.is_empty() {
                // Advance iterators
                for _ in 0..a_num_str.len() {
                    a_chars.next();
                }
                for _ in 0..b_num_str.len() {
                    b_chars.next();
                }

                // Numeric comparison
                let a_num = a_num_str.parse::<u64>().unwrap_or(0);
                let b_num = b_num_str.parse::<u64>().unwrap_or(0);

                match a_num.cmp(&b_num) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }

            // Check if both exhausted
            if a_chars.peek().is_none() && b_chars.peek().is_none() {
                return Ordering::Equal;
            }

            if a_chars.peek().is_none() {
                return Ordering::Less;
            }

            if b_chars.peek().is_none() {
                return Ordering::Greater;
            }
        }
    }
}

impl PartialOrd for EvrString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EvrString {
    fn cmp(&self, other: &Self) -> Ordering {
        // 1. Compare epochs (higher wins)
        match (self.epoch, other.epoch) {
            (Some(a), Some(b)) => {
                match a.cmp(&b) {
                    Ordering::Equal => {}
                    other => return other,
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => {}
        }

        // 2. Compare versions
        match Self::compare_segments(&self.version, &other.version) {
            Ordering::Equal => {}
            other => return other,
        }

        // 3. Compare releases
        match (&self.release, &other.release) {
            (Some(a), Some(b)) => Self::compare_segments(a, b),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EvrParseError {
    #[error("Empty EVR string")]
    EmptyString,

    #[error("Invalid epoch: {0}")]
    InvalidEpoch(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evr_parsing() {
        let evr = EvrString::parse("2:1.0.0-27.el9").unwrap();
        assert_eq!(evr.epoch, Some(2));
        assert_eq!(evr.version, "1.0.0");
        assert_eq!(evr.release, Some("27.el9".to_string()));
    }

    #[test]
    fn test_evr_epoch_precedence() {
        let v1 = EvrString::parse("2:1.0.0").unwrap();
        let v2 = EvrString::parse("9.9.9").unwrap();
        assert!(v1 > v2); // Epoch wins
    }

    #[test]
    fn test_evr_numeric_comparison() {
        let v1 = EvrString::parse("1.0.10").unwrap();
        let v2 = EvrString::parse("1.0.9").unwrap();
        assert!(v1 > v2); // Numeric not lexicographic
    }
}
```

#### 5.2 EVR Comparison Module

**File:** `esp_scanner_base/src/execution/comparisons/evr.rs` (NEW)

```rust
//! EVR string comparison operations

use super::ComparisonError;
use crate::types::common::Operation;
use crate::types::evr::EvrString;

pub fn compare_evr(
    actual: &str,
    expected: &str,
    operation: Operation,
) -> Result<bool, ComparisonError> {
    let actual_evr = EvrString::parse(actual)
        .map_err(|e| ComparisonError::TypeMismatch {
            expected: "evr_string".to_string(),
            actual: format!("Invalid EVR: {}", e),
        })?;

    let expected_evr = EvrString::parse(expected)
        .map_err(|e| ComparisonError::TypeMismatch {
            expected: "evr_string".to_string(),
            actual: format!("Invalid EVR: {}", e),
        })?;

    Ok(match operation {
        Operation::Equals => actual_evr == expected_evr,
        Operation::NotEqual => actual_evr != expected_evr,
        Operation::GreaterThan => actual_evr > expected_evr,
        Operation::LessThan => actual_evr < expected_evr,
        Operation::GreaterThanOrEqual => actual_evr >= expected_evr,
        Operation::LessThanOrEqual => actual_evr <= expected_evr,
        _ => {
            return Err(ComparisonError::UnsupportedOperation {
                operation: format!("{:?}", operation),
                data_type: "evr_string".to_string(),
            })
        }
    })
}
```

#### 5.3 Integration

**File:** `esp_scanner_base/src/execution/comparisons/mod.rs`

```rust
pub mod evr;

pub fn compare_values(
    expected: &ResolvedValue,
    actual: &ResolvedValue,
    operation: Operation,
) -> Result<bool, ComparisonError> {
    match (expected, actual) {
        // ... existing comparisons ...

        (ResolvedValue::EvrString(exp), ResolvedValue::EvrString(act)) => {
            evr::compare_evr(act, exp, operation)
        }

        // ... rest ...
    }
}
```

---

## Implementation Timeline

### Week 1: SET Operations & Filters

- **Day 1-2:** Fix SET expansion borrow checker issues
- **Day 3:** Complete SET expansion logic
- **Day 4:** Complete FILTER evaluation
- **Day 5:** Integration testing

### Week 2: BEHAVIOR & Patterns

- **Day 6-7:** BEHAVIOR framework in contracts
- **Day 8:** Pattern matching implementation
- **Day 9:** Integration and testing
- **Day 10:** Buffer for issues

### Week 3: EVR & Polish

- **Day 11-13:** EVR comparison implementation
- **Day 14-15:** End-to-end testing
- **Day 16:** Documentation and examples

---

## Testing Strategy

### Unit Tests Required

```rust
// SET expansion tests
#[test]
fn test_set_union_expansion() { }

#[test]
fn test_set_nested_refs() { }

#[test]
fn test_set_circular_detection() { }

// Filter tests
#[test]
fn test_filter_include_satisfied() { }

#[test]
fn test_filter_exclude_satisfied() { }

#[test]
fn test_object_filter_application() { }

// Pattern tests
#[test]
fn test_pattern_match_valid() { }

#[test]
fn test_pattern_match_invalid_regex() { }

// EVR tests
#[test]
fn test_evr_epoch_wins() { }

#[test]
fn test_evr_numeric_segments() { }
```

### Integration Test ESP Files

Create these test files to validate end-to-end:

1. **set_expansion_test.esp** - Tests SET operations
2. **filter_test.esp** - Tests FILTER specs
3. **behavior_test.esp** - Tests BEHAVIOR hints
4. **pattern_test.esp** - Tests pattern matching
5. **evr_test.esp** - Tests EVR comparisons
