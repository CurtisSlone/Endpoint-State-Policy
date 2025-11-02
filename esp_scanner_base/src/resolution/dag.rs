//! Dependency Graph (DAG) implementation for ICS resolution engine
//! Handles topological sorting and cycle detection for symbol resolution

use crate::ffi::logging::{
    consumer_codes, log_consumer_debug, log_consumer_error, log_consumer_warning,
};
use crate::resolution::error::ResolutionError;
use crate::types::criterion::CtnNodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Types of symbols that can be resolved in the DAG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)] // Already has Clone
pub enum SymbolType {
    Variable,
    GlobalState,
    GlobalObject,
    SetOperation,
    RuntimeOperation,
    LocalState,
    LocalObject,
}

impl SymbolType {
    /// Convert from string representation (for JSON parsing)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Variable" => Some(Self::Variable),
            "GlobalState" => Some(Self::GlobalState),
            "GlobalObject" => Some(Self::GlobalObject),
            "SetOperation" => Some(Self::SetOperation),
            "RuntimeOperation" => Some(Self::RuntimeOperation),
            "LocalState" => Some(Self::LocalState),
            "LocalObject" => Some(Self::LocalObject),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Variable => "Variable",
            Self::GlobalState => "GlobalState",
            Self::GlobalObject => "GlobalObject",
            Self::SetOperation => "SetOperation",
            Self::RuntimeOperation => "RuntimeOperation",
            Self::LocalState => "LocalState",
            Self::LocalObject => "LocalObject",
        }
    }

    /// Check if this symbol type is global scope
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            Self::Variable
                | Self::GlobalState
                | Self::GlobalObject
                | Self::SetOperation
                | Self::RuntimeOperation
        )
    }

    /// Check if this symbol type is local (CTN) scope
    pub fn is_local(&self) -> bool {
        matches!(self, Self::LocalState | Self::LocalObject)
    }
}

/// Node in the dependency graph representing a resolvable symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)] // Added Clone
pub struct SymbolNode {
    pub symbol_id: String,
    pub symbol_type: SymbolType,
    pub ctn_context: Option<CtnNodeId>,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

impl SymbolNode {
    /// Create new symbol node
    pub fn new(symbol_id: String, symbol_type: SymbolType) -> Self {
        Self {
            symbol_id,
            symbol_type,
            ctn_context: None,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    /// Create new local symbol node with CTN context
    pub fn new_local(symbol_id: String, symbol_type: SymbolType, ctn_id: CtnNodeId) -> Self {
        Self {
            symbol_id,
            symbol_type,
            ctn_context: Some(ctn_id),
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    /// Add dependency to this node
    pub fn add_dependency(&mut self, dependency: String) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
    }

    /// Add dependent to this node
    pub fn add_dependent(&mut self, dependent: String) {
        if !self.dependents.contains(&dependent) {
            self.dependents.push(dependent);
        }
    }

    /// Check if node has dependencies
    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    /// Get dependency count
    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }
}

/// Dependency graph for symbol resolution ordering
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: HashMap<String, SymbolNode>,
    pub edges: HashMap<String, Vec<String>>, // symbol -> its dependencies
    pub reverse_edges: HashMap<String, Vec<String>>, // symbol -> symbols that depend on it
    pub global_symbols: HashSet<String>,
    pub local_symbols: HashMap<CtnNodeId, HashSet<String>>,
}

impl DependencyGraph {
    /// Create new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            global_symbols: HashSet::new(),
            local_symbols: HashMap::new(),
        }
    }

    /// Add node to the dependency graph
    pub fn add_node(
        &mut self,
        symbol_id: String,
        symbol_type: SymbolType,
    ) -> Result<(), ResolutionError> {
        let _ = log_consumer_debug(
            "Adding node to dependency graph",
            &[
                ("symbol_id", &symbol_id),
                ("symbol_type", symbol_type.as_str()),
                ("is_global", &symbol_type.is_global().to_string()),
            ],
        );

        if self.nodes.contains_key(&symbol_id) {
            let _ = log_consumer_warning(
                &format!(
                    "Symbol '{}' already exists in dependency graph - skipping",
                    symbol_id
                ),
                &[("symbol_id", &symbol_id)],
            );
            return Ok(());
        }

        let node = SymbolNode::new(symbol_id.clone(), symbol_type);

        // Track global vs local symbols
        if symbol_type.is_global() {
            self.global_symbols.insert(symbol_id.clone());
        }

        // Initialize edge lists
        self.edges.insert(symbol_id.clone(), Vec::new());
        self.reverse_edges.insert(symbol_id.clone(), Vec::new());

        // Store the node
        self.nodes.insert(symbol_id.clone(), node);

        let _ = log_consumer_debug(
            "Node added to dependency graph successfully",
            &[("symbol_id", &symbol_id)],
        );

        Ok(())
    }

    /// Add local node with CTN context
    pub fn add_local_node(
        &mut self,
        symbol_id: String,
        symbol_type: SymbolType,
        ctn_id: CtnNodeId,
    ) -> Result<(), ResolutionError> {
        let _ = log_consumer_debug(
            "Adding local node to dependency graph",
            &[
                ("symbol_id", &symbol_id),
                ("symbol_type", symbol_type.as_str()),
                ("ctn_id", &ctn_id.to_string()),
            ],
        );

        if self.nodes.contains_key(&symbol_id) {
            return Err(ResolutionError::LocalSymbolConflict {
                symbol: symbol_id,
                ctn_id,
            });
        }

        let node = SymbolNode::new_local(symbol_id.clone(), symbol_type, ctn_id);

        // Track local symbols by CTN
        self.local_symbols
            .entry(ctn_id)
            .or_insert_with(HashSet::new)
            .insert(symbol_id.clone());

        // Initialize edge lists
        self.edges.insert(symbol_id.clone(), Vec::new());
        self.reverse_edges.insert(symbol_id.clone(), Vec::new());

        // Store the node
        self.nodes.insert(symbol_id.clone(), node);

        let _ = log_consumer_debug(
            "Local node added to dependency graph successfully",
            &[("symbol_id", &symbol_id), ("ctn_id", &ctn_id.to_string())],
        );

        Ok(())
    }

    /// Add dependency edge between two symbols
    /// `from` depends on `to` - meaning `to` must be resolved before `from`
    pub fn add_dependency(&mut self, from: &str, to: &str) -> Result<(), ResolutionError> {
        let _ = log_consumer_debug(
            "Adding dependency edge - '{}' depends on '{}'",
            &[("dependent", from), ("dependency", to)],
        );

        // Validate both symbols exist
        if !self.nodes.contains_key(from) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!(
                    "Cannot add dependency: dependent symbol '{}' not found in graph",
                    from
                ),
                &[("dependent", from), ("dependency", to)],
            );
            return Err(ResolutionError::DependencyGraphCorrupted {
                details: format!(
                    "Dependent symbol '{}' not found when adding dependency on '{}'",
                    from, to
                ),
            });
        }

        if !self.nodes.contains_key(to) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!(
                    "Cannot add dependency: dependency symbol '{}' not found in graph",
                    to
                ),
                &[("dependent", from), ("dependency", to)],
            );
            return Err(ResolutionError::DependencyGraphCorrupted {
                details: format!(
                    "Dependency symbol '{}' not found when adding dependency from '{}'",
                    to, from
                ),
            });
        }

        // Add edge: from depends on to (to must resolve first)
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());

        // Add reverse edge: to is depended on by from
        self.reverse_edges
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());

        // Update node dependency lists
        if let Some(node) = self.nodes.get_mut(from) {
            node.add_dependency(to.to_string());
        }
        if let Some(node) = self.nodes.get_mut(to) {
            node.add_dependent(from.to_string());
        }

        let _ = log_consumer_debug(
            "Dependency edge added successfully - '{}' now depends on '{}'",
            &[("dependent", from), ("dependency", to)],
        );

        Ok(())
    }

    /// Get dependencies for a symbol
    pub fn get_dependencies(&self, symbol: &str) -> Vec<String> {
        self.edges.get(symbol).cloned().unwrap_or_default()
    }

    /// Get dependents for a symbol
    pub fn get_dependents(&self, symbol: &str) -> Vec<String> {
        self.reverse_edges.get(symbol).cloned().unwrap_or_default()
    }

    /// Perform topological sort to get resolution order
    pub fn topological_sort(&self) -> Result<Vec<String>, ResolutionError> {
        let _ = log_consumer_debug(
            "Starting topological sort of dependency graph",
            &[
                ("total_nodes", &self.nodes.len().to_string()),
                (
                    "total_edges",
                    &self
                        .edges
                        .values()
                        .map(|v| v.len())
                        .sum::<usize>()
                        .to_string(),
                ),
            ],
        );

        // Detect cycles first
        if let Some(cycle) = self.detect_cycle() {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!(
                    "Circular dependency detected in resolution graph: {}",
                    cycle.join(" -> ")
                ),
                &[("cycle_length", &cycle.len().to_string())],
            );
            return Err(ResolutionError::CircularDependency { cycle });
        }

        // Kahn's algorithm for topological sorting
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Initialize in-degrees
        for node_id in self.nodes.keys() {
            let degree = self.get_dependencies(node_id).len();
            in_degree.insert(node_id.clone(), degree);

            if degree == 0 {
                queue.push_back(node_id.clone());
                let _ = log_consumer_debug(
                    "Node with no dependencies added to queue",
                    &[("symbol_id", node_id)],
                );
            }
        }

        let _ = log_consumer_debug(
            "Topological sort initialization complete",
            &[
                ("nodes_with_no_deps", &queue.len().to_string()),
                ("total_nodes_to_process", &in_degree.len().to_string()),
            ],
        );

        // Process nodes with no remaining dependencies
        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            let _ = log_consumer_debug(
                "Processing node in topological sort",
                &[
                    ("current_node", &current),
                    ("processed_count", &result.len().to_string()),
                ],
            );

            // Update in-degrees for dependents
            for dependent in self.get_dependents(&current) {
                if let Some(degree) = in_degree.get_mut(&dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                        let _ = log_consumer_debug(
                            "Dependent node ready for processing",
                            &[("dependent_node", &dependent)],
                        );
                    }
                }
            }
        }

        // Verify all nodes were processed
        if result.len() != self.nodes.len() {
            let unprocessed: Vec<String> = self
                .nodes
                .keys()
                .filter(|k| !result.contains(k))
                .cloned()
                .collect();

            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!(
                    "Topological sort failed: {} nodes remain unprocessed",
                    unprocessed.len()
                ),
                &[
                    ("unprocessed_count", &unprocessed.len().to_string()),
                    ("unprocessed_nodes", &unprocessed.join(", ")),
                ],
            );

            return Err(ResolutionError::DependencyGraphCorrupted {
                details: format!(
                    "Topological sort incomplete: {} unprocessed nodes",
                    unprocessed.len()
                ),
            });
        }

        let _ = log_consumer_debug(
            "Topological sort completed successfully",
            &[
                ("resolution_order_length", &result.len().to_string()),
                (
                    "first_5_nodes",
                    &result
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ],
        );

        Ok(result)
    }

    /// Detect cycles in the dependency graph using DFS
    pub fn detect_cycle(&self) -> Option<Vec<String>> {
        let _ = log_consumer_debug(
            "Starting cycle detection",
            &[("total_nodes", &self.nodes.len().to_string())],
        );

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if let Some(cycle) =
                    self.dfs_cycle_detect(node_id, &mut visited, &mut rec_stack, &mut path)
                {
                    let _ = log_consumer_debug(
                        "Cycle detected during DFS",
                        &[
                            ("cycle_start", node_id),
                            ("cycle_length", &cycle.len().to_string()),
                        ],
                    );
                    return Some(cycle);
                }
            }
        }

        let _ = log_consumer_debug("Cycle detection completed - no cycles found", &[]);
        None
    }

    /// DFS-based cycle detection helper
    fn dfs_cycle_detect(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        // Follow dependencies (outgoing edges)
        for dependency in self.get_dependencies(node) {
            if !visited.contains(&dependency) {
                if let Some(cycle) = self.dfs_cycle_detect(&dependency, visited, rec_stack, path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(&dependency) {
                // Back edge found - cycle detected
                // Extract cycle from path starting at the back edge target
                if let Some(cycle_start_idx) = path.iter().position(|x| x == &dependency) {
                    let mut cycle = path[cycle_start_idx..].to_vec();
                    cycle.push(dependency);
                    return Some(cycle);
                } else {
                    // Fallback: just return the current path as cycle
                    let mut cycle = path.clone();
                    cycle.push(dependency);
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Get graph statistics for monitoring
    pub fn get_stats(&self) -> GraphStats {
        let edge_count: usize = self.edges.values().map(|v| v.len()).sum();
        let max_dependencies = self.edges.values().map(|v| v.len()).max().unwrap_or(0);
        let nodes_with_no_deps = self.edges.values().filter(|v| v.is_empty()).count();

        GraphStats {
            total_nodes: self.nodes.len(),
            total_edges: edge_count,
            global_symbols: self.global_symbols.len(),
            local_symbols: self.local_symbols.values().map(|s| s.len()).sum(),
            max_dependencies,
            nodes_with_no_dependencies: nodes_with_no_deps,
        }
    }

    /// Validate graph integrity
    pub fn validate(&self) -> Result<(), ResolutionError> {
        let _ = log_consumer_debug(
            "Validating dependency graph integrity",
            &[("total_nodes", &self.nodes.len().to_string())],
        );

        // Check that all edges reference existing nodes
        for (from, dependencies) in &self.edges {
            if !self.nodes.contains_key(from) {
                return Err(ResolutionError::DependencyGraphCorrupted {
                    details: format!("Edge references non-existent source node: {}", from),
                });
            }

            for to in dependencies {
                if !self.nodes.contains_key(to) {
                    return Err(ResolutionError::DependencyGraphCorrupted {
                        details: format!(
                            "Edge from '{}' references non-existent target node: {}",
                            from, to
                        ),
                    });
                }
            }
        }

        // Check reverse edge consistency
        for (node, dependents) in &self.reverse_edges {
            for dependent in dependents {
                let forward_deps = self.edges.get(dependent);
                if !forward_deps.map_or(false, |deps| deps.contains(node)) {
                    return Err(ResolutionError::DependencyGraphCorrupted {
                        details: format!("Inconsistent reverse edge: {} -> {}", dependent, node),
                    });
                }
            }
        }

        let _ = log_consumer_debug("Dependency graph validation passed", &[]);
        Ok(())
    }

    /// Get symbols that have no dependencies (can be resolved first)
    pub fn get_independent_symbols(&self) -> Vec<String> {
        self.edges
            .iter()
            .filter_map(|(symbol, deps)| {
                if deps.is_empty() {
                    Some(symbol.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get symbols that nothing depends on (leaf nodes)
    pub fn get_leaf_symbols(&self) -> Vec<String> {
        self.reverse_edges
            .iter()
            .filter_map(|(symbol, dependents)| {
                if dependents.is_empty() {
                    Some(symbol.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Graph statistics for monitoring and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub global_symbols: usize,
    pub local_symbols: usize,
    pub max_dependencies: usize,
    pub nodes_with_no_dependencies: usize,
}

impl GraphStats {
    /// Calculate average dependencies per node
    pub fn average_dependencies(&self) -> f64 {
        if self.total_nodes == 0 {
            0.0
        } else {
            self.total_edges as f64 / self.total_nodes as f64
        }
    }

    /// Check if graph is well-balanced (not too many dependencies per node)
    pub fn is_well_balanced(&self) -> bool {
        self.max_dependencies <= 10 && self.average_dependencies() <= 3.0
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_topological_sort() {
        let mut graph = DependencyGraph::new();

        // Add nodes: A depends on B, B depends on C
        // Resolution order should be: C, B, A
        graph
            .add_node("A".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("B".to_string(), SymbolType::RuntimeOperation)
            .unwrap();
        graph
            .add_node("C".to_string(), SymbolType::SetOperation)
            .unwrap();

        // A depends on B (B must resolve before A)
        graph.add_dependency("A", "B").unwrap();
        // B depends on C (C must resolve before B)
        graph.add_dependency("B", "C").unwrap();

        let order = graph.topological_sort().unwrap();

        // C should be first (no dependencies), then B, then A
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let a_pos = order.iter().position(|x| x == "A").unwrap();

        assert!(c_pos < b_pos, "C should come before B");
        assert!(b_pos < a_pos, "B should come before A");
    }

    #[test]
    fn test_independent_nodes() {
        let mut graph = DependencyGraph::new();

        // Add independent nodes with no dependencies
        graph
            .add_node("X".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("Y".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("Z".to_string(), SymbolType::Variable)
            .unwrap();

        let order = graph.topological_sort().unwrap();

        // All nodes should be present, order can be any
        assert_eq!(order.len(), 3);
        assert!(order.contains(&"X".to_string()));
        assert!(order.contains(&"Y".to_string()));
        assert!(order.contains(&"Z".to_string()));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node("A".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("B".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("C".to_string(), SymbolType::Variable)
            .unwrap();

        // Create cycle: A -> B -> C -> A
        graph.add_dependency("A", "B").unwrap();
        graph.add_dependency("B", "C").unwrap();
        graph.add_dependency("C", "A").unwrap();

        assert!(graph.detect_cycle().is_some());
    }

    #[test]
    fn test_no_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node("A".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("B".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("C".to_string(), SymbolType::Variable)
            .unwrap();

        // No cycle: A -> B, A -> C
        graph.add_dependency("A", "B").unwrap();
        graph.add_dependency("A", "C").unwrap();

        assert!(graph.detect_cycle().is_none());
    }

    #[test]
    fn test_complex_dependency_resolution() {
        let mut graph = DependencyGraph::new();

        // Create diamond dependency pattern:
        // A depends on both B and C
        // B depends on D
        // C depends on D
        // D has no dependencies
        // Resolution order should be: D, then B and C (any order), then A
        for symbol in ["A", "B", "C", "D"] {
            graph
                .add_node(symbol.to_string(), SymbolType::Variable)
                .unwrap();
        }

        graph.add_dependency("A", "B").unwrap(); // A depends on B
        graph.add_dependency("A", "C").unwrap(); // A depends on C
        graph.add_dependency("B", "D").unwrap(); // B depends on D
        graph.add_dependency("C", "D").unwrap(); // C depends on D

        let order = graph.topological_sort().unwrap();

        // Get positions
        let a_pos = order.iter().position(|x| x == "A").unwrap();
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        let d_pos = order.iter().position(|x| x == "D").unwrap();

        // D should be first (no dependencies)
        assert_eq!(d_pos, 0, "D should resolve first");

        // B and C should come before A
        assert!(b_pos < a_pos, "B should resolve before A");
        assert!(c_pos < a_pos, "C should resolve before A");

        // B and C should come after D
        assert!(d_pos < b_pos, "D should resolve before B");
        assert!(d_pos < c_pos, "D should resolve before C");

        // A should be last
        assert_eq!(a_pos, 3, "A should resolve last");
    }

    #[test]
    fn test_graph_validation() {
        let mut graph = DependencyGraph::new();

        graph
            .add_node("A".to_string(), SymbolType::Variable)
            .unwrap();
        graph
            .add_node("B".to_string(), SymbolType::Variable)
            .unwrap();
        graph.add_dependency("A", "B").unwrap();

        // Should pass validation
        assert!(graph.validate().is_ok());
    }
}
