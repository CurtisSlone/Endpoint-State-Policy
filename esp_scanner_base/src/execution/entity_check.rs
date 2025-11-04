//! # Entity Check Collection Support
//!
//! Utilities for collectors to properly handle entity checks by returning
//! multiple values when needed.

use crate::types::common::ResolvedValue;
use crate::types::execution_context::ExecutableCriterion;
use std::collections::{HashMap, HashSet};

/// Helper to identify which fields need entity-level collection
///
/// This allows collectors to determine which fields should return
/// ResolvedValue::Collection vs single values.
pub struct EntityCheckAnalyzer;

impl EntityCheckAnalyzer {
    /// Analyze criterion to find fields requiring entity-level collection
    ///
    /// Returns a map of field_name -> entity_check_type
    pub fn get_entity_check_fields(
        criterion: &ExecutableCriterion,
    ) -> HashMap<String, crate::types::EntityCheck> {
        let mut entity_fields = HashMap::new();

        // Check all states for entity checks
        for state in &criterion.states {
            for field in &state.fields {
                if let Some(entity_check) = field.entity_check {
                    entity_fields.insert(field.name.clone(), entity_check);
                }
            }

            // Also check record fields
            for record_check in &state.record_checks {
                if let crate::types::execution_context::ExecutableRecordContent::Nested { fields } =
                    &record_check.content
                {
                    for record_field in fields {
                        if let Some(entity_check) = record_field.entity_check {
                            // Use dot notation for record field names
                            entity_fields.insert(record_field.path.to_dot_notation(), entity_check);
                        }
                    }
                }
            }
        }

        entity_fields
    }

    /// Check if a specific field requires entity-level collection
    pub fn field_needs_entity_collection(
        criterion: &ExecutableCriterion,
        field_name: &str,
    ) -> bool {
        criterion.states.iter().any(|state| {
            state
                .fields
                .iter()
                .any(|field| field.name == field_name && field.entity_check.is_some())
        })
    }

    /// Get all field names that need entity collection (no duplicates)
    pub fn get_entity_field_names(criterion: &ExecutableCriterion) -> HashSet<String> {
        let entity_fields = Self::get_entity_check_fields(criterion);
        entity_fields.keys().cloned().collect()
    }
}

/// Extension trait for collectors to easily handle entity checks
pub trait EntityCheckCollector {
    /// Collect a field value, returning Collection if entity check is present
    ///
    /// # Arguments
    /// * `field_name` - Name of the field to collect
    /// * `criterion` - The criterion being executed (to check for entity checks)
    ///
    /// # Returns
    /// * Single value if no entity check
    /// * Collection of values if entity check present
    fn collect_field_with_entity_awareness(
        &self,
        field_name: &str,
        criterion: &ExecutableCriterion,
    ) -> Result<ResolvedValue, crate::strategies::CollectionError>;
}

/// Helper for wrapping single values as collections when needed
pub fn wrap_for_entity_check(value: ResolvedValue, needs_collection: bool) -> ResolvedValue {
    if needs_collection {
        match value {
            ResolvedValue::Collection(_) => value, // Already a collection
            single_value => ResolvedValue::Collection(vec![single_value]),
        }
    } else {
        value
    }
}

/// Helper for collectors: determine collection strategy for object
///
/// Returns (field_name, should_return_collection) pairs
pub fn get_collection_strategy(
    criterion: &ExecutableCriterion,
    _object_id: &str,
) -> HashMap<String, bool> {
    let entity_fields = EntityCheckAnalyzer::get_entity_check_fields(criterion);

    // For each field in the object, check if it needs collection
    let mut strategy = HashMap::new();

    // Get all field names from criterion states
    for state in &criterion.states {
        for field in &state.fields {
            let needs_collection = entity_fields.contains_key(&field.name);
            strategy.insert(field.name.clone(), needs_collection);
        }
    }

    strategy
}

/// Example collector implementation with entity check support
#[cfg(test)]
mod example_collector {
    use super::*;
    use crate::strategies::{CollectedData, CollectionError, DataCollector};
    use crate::types::execution_context::ExecutableObject;

    pub struct ExampleEntityAwareCollector;

    impl DataCollector for ExampleEntityAwareCollector {
        fn collect(&self, object: &ExecutableObject) -> Result<CollectedData, CollectionError> {
            let mut data = CollectedData::new(object.identifier.clone());

            // This would normally be passed from executor context
            // For now, we'd need to extract entity check info from object somehow
            // In practice, executors will pass this context to collectors

            // Example: collect with entity awareness
            // if entity_fields.contains(&field_name) {
            //     let values = self.collect_multiple_values(&field_name)?;
            //     data.add_field(field_name, ResolvedValue::Collection(values));
            // } else {
            //     let value = self.collect_single_value(&field_name)?;
            //     data.add_field(field_name, value);
            // }

            Ok(data)
        }

        fn extract_field(
            &self,
            data: &CollectedData,
            field_path: &str,
        ) -> Result<ResolvedValue, CollectionError> {
            data.get_field(field_path)
                .cloned()
                .ok_or_else(|| CollectionError::FieldNotAccessible {
                    object_id: data.object_id.clone(),
                    field_name: field_path.to_string(),
                })
        }

        fn collector_type(&self) -> &str {
            "example_entity_aware"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::common::*;
    use crate::types::execution_context::*;
    use crate::types::state::EntityCheck;
    use crate::types::test::*;

    fn create_test_criterion_with_entity_check() -> ExecutableCriterion {
        let field_with_entity_check = ExecutableStateField {
            name: "test_field".to_string(),
            data_type: DataType::String,
            operation: Operation::Equals,
            value: ResolvedValue::String("test".to_string()),
            entity_check: Some(EntityCheck::All),
        };

        let field_without_entity_check = ExecutableStateField {
            name: "normal_field".to_string(),
            data_type: DataType::String,
            operation: Operation::Equals,
            value: ResolvedValue::String("test".to_string()),
            entity_check: None,
        };

        let state = ExecutableState {
            identifier: "test_state".to_string(),
            fields: vec![field_with_entity_check, field_without_entity_check],
            record_checks: vec![],
            source: StateSource::Local,
            is_valid: true,
        };

        ExecutableCriterion {
            criterion_type: "test".to_string(),
            test: TestSpecification::simple(ExistenceCheck::Any, ItemCheck::All),
            ctn_node_id: 0,
            states: vec![state],
            objects: vec![],
            referenced_global_states: HashMap::new(),
            referenced_global_objects: HashMap::new(),
        }
    }

    #[test]
    fn test_entity_check_field_detection() {
        let criterion = create_test_criterion_with_entity_check();
        let entity_fields = EntityCheckAnalyzer::get_entity_check_fields(&criterion);

        assert!(entity_fields.contains_key("test_field"));
        assert!(!entity_fields.contains_key("normal_field"));
        assert_eq!(entity_fields.get("test_field"), Some(&EntityCheck::All));
    }

    #[test]
    fn test_field_needs_entity_collection() {
        let criterion = create_test_criterion_with_entity_check();

        assert!(EntityCheckAnalyzer::field_needs_entity_collection(
            &criterion,
            "test_field"
        ));
        assert!(!EntityCheckAnalyzer::field_needs_entity_collection(
            &criterion,
            "normal_field"
        ));
    }

    #[test]
    fn test_wrap_for_entity_check() {
        let single_value = ResolvedValue::String("test".to_string());

        // Without entity check - returns single value
        let result = wrap_for_entity_check(single_value.clone(), false);
        assert!(matches!(result, ResolvedValue::String(_)));

        // With entity check - wraps in collection
        let result = wrap_for_entity_check(single_value.clone(), true);
        assert!(matches!(result, ResolvedValue::Collection(_)));
    }

    #[test]
    fn test_get_entity_field_names() {
        let criterion = create_test_criterion_with_entity_check();
        let field_names = EntityCheckAnalyzer::get_entity_field_names(&criterion);

        assert_eq!(field_names.len(), 1);
        assert!(field_names.contains("test_field"));
    }
}
