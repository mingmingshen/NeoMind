//! Rule dependency management for execution ordering.
//!
//! This module provides functionality for managing rule dependencies,
//! including topological sorting and circular dependency detection.

use crate::engine::RuleId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Dependency relationship between rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleDependency {
    /// ID of the rule that depends on another.
    pub rule_id: RuleId,
    /// ID of the rule this rule depends on.
    pub depends_on: RuleId,
    /// Type of dependency.
    pub dependency_type: DependencyType,
}

/// Type of dependency between rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyType {
    /// The dependent rule must execute after the dependency.
    ExecutesAfter,
    /// The dependent rule requires the dependency to have executed successfully.
    RequiresSuccess,
    /// The dependent rule requires data from the dependency.
    RequiresData,
    /// Custom dependency type.
    Custom(u8),
}

/// Result of dependency validation.
#[derive(Debug, Clone)]
pub struct DependencyValidationResult {
    /// Whether dependencies are valid (no circular dependencies).
    pub is_valid: bool,
    /// Execution order (topologically sorted rule IDs).
    pub execution_order: Vec<RuleId>,
    /// Circular dependencies found (if any).
    pub circular_dependencies: Vec<Vec<RuleId>>,
    /// Missing dependencies (rules that don't exist).
    pub missing_dependencies: Vec<RuleId>,
}

impl DependencyValidationResult {
    /// Create a new valid result.
    pub fn valid(execution_order: Vec<RuleId>) -> Self {
        Self {
            is_valid: true,
            execution_order,
            circular_dependencies: Vec::new(),
            missing_dependencies: Vec::new(),
        }
    }

    /// Create a new invalid result with circular dependencies.
    pub fn circular(circular_dependencies: Vec<Vec<RuleId>>) -> Self {
        Self {
            is_valid: false,
            execution_order: Vec::new(),
            circular_dependencies,
            missing_dependencies: Vec::new(),
        }
    }

    /// Create a new invalid result with missing dependencies.
    pub fn missing(missing_dependencies: Vec<RuleId>) -> Self {
        Self {
            is_valid: false,
            execution_order: Vec::new(),
            circular_dependencies: Vec::new(),
            missing_dependencies,
        }
    }

    /// Format the result as a human-readable message.
    pub fn format_message(&self) -> String {
        if self.is_valid {
            format!(
                "Dependencies are valid. Execution order: {} rules.",
                self.execution_order.len()
            )
        } else {
            let mut msg = "Invalid dependencies:".to_string();

            if !self.circular_dependencies.is_empty() {
                msg.push_str("\n  Circular dependencies detected:");
                for cycle in &self.circular_dependencies {
                    let cycle_str: Vec<String> = cycle.iter().map(|id| id.to_string()).collect();
                    msg.push_str(&format!("\n    - {}", cycle_str.join(" -> ")));
                }
            }

            if !self.missing_dependencies.is_empty() {
                msg.push_str("\n  Missing dependencies:");
                for missing in &self.missing_dependencies {
                    msg.push_str(&format!("\n    - {}", missing));
                }
            }

            msg
        }
    }
}

/// Manager for rule dependencies.
pub struct DependencyManager {
    /// Map of rule ID to its dependencies.
    dependencies: HashMap<RuleId, HashSet<RuleId>>,
    /// Reverse dependency map (for efficient lookup).
    reverse_dependencies: HashMap<RuleId, HashSet<RuleId>>,
}

impl Default for DependencyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyManager {
    /// Create a new dependency manager.
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        }
    }

    /// Add a dependency relationship.
    pub fn add_dependency(&mut self, rule_id: RuleId, depends_on: RuleId) {
        self.dependencies
            .entry(rule_id.clone())
            .or_default()
            .insert(depends_on.clone());

        self.reverse_dependencies
            .entry(depends_on)
            .or_default()
            .insert(rule_id);
    }

    /// Remove a dependency relationship.
    pub fn remove_dependency(&mut self, rule_id: &RuleId, depends_on: &RuleId) {
        if let Some(deps) = self.dependencies.get_mut(rule_id) {
            deps.remove(depends_on);
        }

        if let Some(reverse) = self.reverse_dependencies.get_mut(depends_on) {
            reverse.remove(rule_id);
        }
    }

    /// Remove all dependencies for a rule.
    pub fn remove_rule(&mut self, rule_id: &RuleId) {
        // Remove outgoing dependencies
        if let Some(deps) = self.dependencies.remove(rule_id) {
            // Clean up reverse dependencies
            for dep in deps {
                if let Some(reverse) = self.reverse_dependencies.get_mut(&dep) {
                    reverse.remove(rule_id);
                }
            }
        }

        // Remove incoming dependencies
        if let Some(reverse) = self.reverse_dependencies.remove(rule_id) {
            for dependent in reverse {
                if let Some(deps) = self.dependencies.get_mut(&dependent) {
                    deps.remove(rule_id);
                }
            }
        }
    }

    /// Get all dependencies for a rule.
    pub fn get_dependencies(&self, rule_id: &RuleId) -> Vec<RuleId> {
        self.dependencies
            .get(rule_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all rules that depend on this rule.
    pub fn get_dependents(&self, rule_id: &RuleId) -> Vec<RuleId> {
        self.reverse_dependencies
            .get(rule_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Validate dependencies and compute execution order.
    ///
    /// Returns a validation result containing the execution order if valid,
    /// or information about what's wrong (circular dependencies, missing rules).
    pub fn validate_and_order(
        &self,
        existing_rules: &HashSet<RuleId>,
    ) -> DependencyValidationResult {
        // Check for missing dependencies
        let mut missing = Vec::new();
        for deps in self.dependencies.values() {
            for dep in deps {
                if !existing_rules.contains(dep) {
                    missing.push(dep.clone());
                }
            }
        }

        if !missing.is_empty() {
            return DependencyValidationResult::missing(missing);
        }

        // Perform topological sort with cycle detection
        match self.topological_sort(existing_rules) {
            Ok(order) => DependencyValidationResult::valid(order),
            Err(cycles) => DependencyValidationResult::circular(cycles),
        }
    }

    /// Perform topological sort and detect circular dependencies.
    ///
    /// Returns Ok with the sorted order, or Err with detected cycles.
    fn topological_sort(&self, existing_rules: &HashSet<RuleId>) -> Result<Vec<RuleId>, Vec<Vec<RuleId>>> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        let mut cycles = Vec::new();

        // Only sort rules that exist
        for rule_id in existing_rules {
            if !visited.contains(rule_id)
                && let Some(cycle) = self.visit(rule_id, &mut visited, &mut visiting, &mut sorted) {
                    cycles.push(cycle);
                }
        }

        if !cycles.is_empty() {
            Err(cycles)
        } else {
            Ok(sorted)
        }
    }

    /// Recursive depth-first visit for topological sort.
    ///
    /// Returns Some(cycle) if a circular dependency is detected.
    fn visit(
        &self,
        rule_id: &RuleId,
        visited: &mut HashSet<RuleId>,
        visiting: &mut HashSet<RuleId>,
        sorted: &mut Vec<RuleId>,
    ) -> Option<Vec<RuleId>> {
        if visited.contains(rule_id) {
            return None;
        }

        if visiting.contains(rule_id) {
            // Found a cycle - return it
            return Some(vec![rule_id.clone()]);
        }

        visiting.insert(rule_id.clone());

        // Visit all dependencies
        if let Some(deps) = self.dependencies.get(rule_id) {
            for dep in deps {
                if let Some(mut cycle) = self.visit(dep, visited, visiting, sorted) {
                    if cycle.last() == Some(rule_id) {
                        // We've completed the cycle
                        cycle.push(rule_id.clone());
                        return Some(cycle);
                    }
                    // Otherwise, propagate the cycle up
                    return Some(cycle);
                }
            }
        }

        visiting.remove(rule_id);
        visited.insert(rule_id.clone());
        sorted.push(rule_id.clone());

        None
    }

    /// Get rules that can be executed immediately (no unmet dependencies).
    pub fn get_ready_rules(
        &self,
        existing_rules: &HashSet<RuleId>,
        completed: &HashSet<RuleId>,
    ) -> Vec<RuleId> {
        let mut ready = Vec::new();

        for rule_id in existing_rules {
            // Skip already completed rules
            if completed.contains(rule_id) {
                continue;
            }

            if let Some(deps) = self.dependencies.get(rule_id) {
                // Check if all dependencies are satisfied
                if deps.iter().all(|dep| completed.contains(dep) || !existing_rules.contains(dep)) {
                    ready.push(rule_id.clone());
                }
            } else {
                // No dependencies, ready to execute
                ready.push(rule_id.clone());
            }
        }

        ready
    }

    /// Get the number of dependencies for a rule.
    pub fn dependency_count(&self, rule_id: &RuleId) -> usize {
        self.dependencies
            .get(rule_id)
            .map(|deps| deps.len())
            .unwrap_or(0)
    }

    /// Get all rule IDs that have dependencies.
    pub fn rules_with_dependencies(&self) -> Vec<RuleId> {
        self.dependencies.keys().cloned().collect()
    }

    /// Clear all dependencies.
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.reverse_dependencies.clear();
    }
}

impl fmt::Debug for DependencyManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependencyManager")
            .field("dependency_count", &self.dependencies.len())
            .field("dependencies", &self.dependencies)
            .field("reverse_dependencies", &self.reverse_dependencies)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_dependency() {
        let mut manager = DependencyManager::new();
        let rule1 = RuleId::new();
        let rule2 = RuleId::new();

        manager.add_dependency(rule1.clone(), rule2.clone());

        assert_eq!(manager.get_dependencies(&rule1), vec![rule2.clone()]);
        assert_eq!(manager.get_dependents(&rule2), vec![rule1]);
    }

    #[test]
    fn test_topological_sort_simple() {
        let mut manager = DependencyManager::new();
        let rule1 = RuleId::new();
        let rule2 = RuleId::new();
        let rule3 = RuleId::new();

        // rule3 depends on rule2, rule2 depends on rule1
        manager.add_dependency(rule2.clone(), rule1.clone());
        manager.add_dependency(rule3.clone(), rule2.clone());

        let existing: HashSet<_> = [rule1.clone(), rule2.clone(), rule3.clone()]
            .iter()
            .cloned()
            .collect();

        let result = manager.validate_and_order(&existing);
        assert!(result.is_valid);

        // rule1 should come before rule2, rule2 before rule3
        let pos1 = result.execution_order.iter().position(|id| id == &rule1).unwrap();
        let pos2 = result.execution_order.iter().position(|id| id == &rule2).unwrap();
        let pos3 = result.execution_order.iter().position(|id| id == &rule3).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut manager = DependencyManager::new();
        let rule1 = RuleId::new();
        let rule2 = RuleId::new();
        let rule3 = RuleId::new();

        // Create a cycle: rule1 -> rule2 -> rule3 -> rule1
        manager.add_dependency(rule1.clone(), rule2.clone());
        manager.add_dependency(rule2.clone(), rule3.clone());
        manager.add_dependency(rule3.clone(), rule1.clone());

        let existing: HashSet<_> =
            [rule1.clone(), rule2.clone(), rule3.clone()]
                .iter()
                .cloned()
                .collect();

        let result = manager.validate_and_order(&existing);
        assert!(!result.is_valid);
        assert!(!result.circular_dependencies.is_empty());
    }

    #[test]
    fn test_get_ready_rules() {
        let mut manager = DependencyManager::new();
        let rule1 = RuleId::new();
        let rule2 = RuleId::new();
        let rule3 = RuleId::new();

        manager.add_dependency(rule2.clone(), rule1.clone());
        manager.add_dependency(rule3.clone(), rule2.clone());

        let existing: HashSet<_> = [rule1.clone(), rule2.clone(), rule3.clone()]
            .iter()
            .cloned()
            .collect();
        let completed: HashSet<_> = [].iter().cloned().collect();

        // Only rule1 is ready (no dependencies)
        let ready = manager.get_ready_rules(&existing, &completed);
        assert_eq!(ready, vec![rule1.clone()]);

        // After rule1 completes, rule2 should be ready
        let completed: HashSet<_> = [rule1].iter().cloned().collect();
        let ready = manager.get_ready_rules(&existing, &completed);
        assert_eq!(ready, vec![rule2.clone()]);
    }

    #[test]
    fn test_remove_rule() {
        let mut manager = DependencyManager::new();
        let rule1 = RuleId::new();
        let rule2 = RuleId::new();
        let rule3 = RuleId::new();

        manager.add_dependency(rule2.clone(), rule1.clone());
        manager.add_dependency(rule3.clone(), rule2.clone());

        // Remove rule2
        manager.remove_rule(&rule2);

        // rule2's dependencies should be gone
        assert_eq!(manager.get_dependencies(&rule2), Vec::<RuleId>::new());
        // rule1 should no longer have rule2 as dependent
        assert_eq!(manager.get_dependents(&rule1), Vec::<RuleId>::new());
    }
}
