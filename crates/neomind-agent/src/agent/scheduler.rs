//! Dependency-Aware Tool Scheduling (P2.1)
//!
//! This module implements intelligent tool execution scheduling based on
//! tool relationship metadata. It enables:
//! - Parallel execution of independent tools
//! - Sequential execution of dependent tools
//! - Conflict detection for mutually exclusive tools

use crate::agent::ToolCall;
use crate::error::Result;
use neomind_core::tools::ToolRelationships;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Execution plan for tool calls.
///
/// Groups tools into execution batches where:
/// - Each batch contains independent tools (can run in parallel)
/// - Batches execute sequentially (later batches wait for earlier ones)
#[derive(Debug, Clone)]
pub struct ToolExecutionPlan {
    /// Execution batches (each batch can run in parallel)
    pub batches: Vec<ExecutionBatch>,
    /// Total number of tool calls
    pub total_calls: usize,
}

/// A batch of independent tool calls that can execute in parallel.
#[derive(Debug, Clone)]
pub struct ExecutionBatch {
    /// Tool calls in this batch (independent of each other)
    pub calls: Vec<ToolCall>,
    /// Estimated execution priority (higher = earlier)
    pub priority: f32,
}

/// Dependency graph node for a tool call.
#[derive(Debug, Clone)]
struct DependencyNode {
    /// The tool call
    call: ToolCall,
    /// Tool relationships metadata
    relationships: ToolRelationships,
    /// Indices of tools this depends on (in the original call list)
    dependencies: HashSet<usize>,
    /// Indices of tools that depend on this
    dependents: HashSet<usize>,
}

/// Build an optimized execution plan for tool calls.
///
/// This analyzes tool relationships and creates an execution plan that:
/// 1. Executes independent tools in parallel (for performance)
/// 2. Executes dependent tools in correct order (for correctness)
/// 3. Detects and handles conflicts (mutually exclusive tools)
pub async fn build_execution_plan(
    tool_calls: Vec<ToolCall>,
    tools: &neomind_tools::ToolRegistry,
) -> Result<ToolExecutionPlan> {
    if tool_calls.is_empty() {
        return Ok(ToolExecutionPlan {
            batches: Vec::new(),
            total_calls: 0,
        });
    }

    // Single tool - no dependencies to resolve
    if tool_calls.len() == 1 {
        return Ok(ToolExecutionPlan {
            batches: vec![ExecutionBatch {
                calls: tool_calls,
                priority: 1.0,
            }],
            total_calls: 1,
        });
    }

    // Build dependency graph
    let nodes = build_dependency_graph(&tool_calls, tools).await?;

    // Detect conflicts (mutually exclusive tools)
    let conflicts = detect_conflicts(&nodes);
    if !conflicts.is_empty() {
        // For now, we prioritize the first tool in a conflict pair
        tracing::warn!(
            conflicts = ?conflicts,
            "Detected mutually exclusive tool calls, prioritizing first occurrence"
        );
    }

    // Create execution batches using topological sort
    let batches = create_execution_batches(&nodes, &conflicts);

    Ok(ToolExecutionPlan {
        batches,
        total_calls: tool_calls.len(),
    })
}

/// Build dependency graph from tool calls and tool metadata.
async fn build_dependency_graph(
    tool_calls: &[ToolCall],
    tools: &neomind_tools::ToolRegistry,
) -> Result<Vec<DependencyNode>> {
    let mut nodes = Vec::new();

    // First pass: create nodes with relationships
    for (idx, call) in tool_calls.iter().enumerate() {
        let relationships = get_tool_relationships(&call.name, tools).await?;

        nodes.push(DependencyNode {
            call: call.clone(),
            relationships,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        });
    }

    // Second pass: build dependency edges (collect first, then apply)
    let mut dependency_edges: Vec<(usize, usize)> = Vec::new(); // (dependent, prerequisite)

    for (idx, node) in nodes.iter().enumerate() {
        // Check call_after requirements
        for required_name in &node.relationships.call_after {
            if let Some(required_idx) = find_tool_by_name(required_name, &nodes) {
                dependency_edges.push((idx, required_idx));
            }
        }

        // Check reverse output_to relationships
        for (other_idx, other_node) in nodes.iter().enumerate() {
            if other_idx == idx {
                continue;
            }

            // If other tool outputs to this tool, other must run first
            if other_node.relationships.output_to.contains(&node.call.name) {
                dependency_edges.push((idx, other_idx));
            }
        }
    }

    // Apply dependency edges
    for (dependent_idx, prerequisite_idx) in dependency_edges {
        if dependent_idx < nodes.len() && prerequisite_idx < nodes.len() {
            nodes[dependent_idx].dependencies.insert(prerequisite_idx);
            nodes[prerequisite_idx].dependents.insert(dependent_idx);
        }
    }

    Ok(nodes)
}

/// Get tool relationships from the tool registry.
async fn get_tool_relationships(
    tool_name: &str,
    tools: &neomind_tools::ToolRegistry,
) -> Result<ToolRelationships> {
    // Try to get tool from registry
    if let Some(tool) = tools.get(tool_name) {
        return Ok(tool.definition().relationships);
    }

    // Try simplified name mapping (common pattern: device_control -> control_device)
    let simplified_variants = vec![
        tool_name.replacen("list_", "get_", 1),
        tool_name.replacen("get_", "list_", 1),
        tool_name.replacen("device_", "", 1),
        tool_name.replacen("rule_", "", 1),
        tool_name.replacen("agent_", "", 1),
    ];

    for variant in simplified_variants {
        if let Some(tool) = tools.get(&variant) {
            return Ok(tool.definition().relationships);
        }
    }

    // Default: no relationships
    Ok(ToolRelationships::default())
}

/// Find a tool by name in the nodes list.
fn find_tool_by_name(name: &str, nodes: &[DependencyNode]) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .find(|(_, node)| node.call.name == name)
        .map(|(idx, _)| idx)
}

/// Detect mutually exclusive tool conflicts.
fn detect_conflicts(nodes: &[DependencyNode]) -> HashSet<(usize, usize)> {
    let mut conflicts = HashSet::new();

    for (i, node_i) in nodes.iter().enumerate() {
        for (j, node_j) in nodes.iter().enumerate() {
            if i >= j {
                continue;
            }

            // Check if tools are mutually exclusive
            if node_i.relationships.exclusive_with.contains(&node_j.call.name)
                || node_j.relationships.exclusive_with.contains(&node_i.call.name)
            {
                conflicts.insert((i, j));
            }
        }
    }

    conflicts
}

/// Create execution batches using topological sort.
fn create_execution_batches(
    nodes: &[DependencyNode],
    conflicts: &HashSet<(usize, usize)>,
) -> Vec<ExecutionBatch> {
    let mut batches = Vec::new();
    let mut executed: HashSet<usize> = HashSet::new();
    let mut in_batch: HashSet<usize> = HashSet::new();

    loop {
        // Find all nodes that can be executed next
        let mut ready = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            if executed.contains(&idx) || in_batch.contains(&idx) {
                continue;
            }

            // Check if all dependencies are executed
            let deps_satisfied = node
                .dependencies
                .iter()
                .all(|dep| executed.contains(dep));

            // Check for conflicts with nodes already in this batch
            let has_conflict = in_batch
                .iter()
                .any(|batch_idx| conflicts.contains(&(*batch_idx, idx)) || conflicts.contains(&(idx, *batch_idx)));

            if deps_satisfied && !has_conflict {
                ready.push((idx, node));
            }
        }

        if ready.is_empty() {
            // Either done or circular dependency
            break;
        }

        // Create a new batch with all ready tools
        let batch_calls: Vec<ToolCall> = ready.iter().map(|(_, node)| node.call.clone()).collect();

        // Calculate priority (prefer tools with more dependents)
        let priority = ready
            .iter()
            .map(|(idx, _)| nodes[*idx].dependents.len() as f32)
            .sum::<f32>()
            / ready.len() as f32;

        batches.push(ExecutionBatch {
            calls: batch_calls,
            priority: priority.max(0.1),
        });

        // Mark these as in the current batch
        for (idx, _) in &ready {
            in_batch.insert(*idx);
        }

        // Mark as executed (move to next iteration)
        for idx in in_batch.drain() {
            executed.insert(idx);
        }
    }

    // Handle circular dependencies by adding remaining tools
    for (idx, node) in nodes.iter().enumerate() {
        if !executed.contains(&idx) {
            tracing::warn!(
                tool = %node.call.name,
                "Tool has circular dependencies or unresolved prerequisites, executing anyway"
            );
            batches.push(ExecutionBatch {
                calls: vec![node.call.clone()],
                priority: 0.01, // Lowest priority
            });
        }
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_plan() {
        let plan = ToolExecutionPlan {
            batches: Vec::new(),
            total_calls: 0,
        };
        assert_eq!(plan.total_calls, 0);
        assert!(plan.batches.is_empty());
    }

    #[test]
    fn test_execution_batch_creation() {
        let batch = ExecutionBatch {
            calls: vec![],
            priority: 1.0,
        };
        assert_eq!(batch.priority, 1.0);
    }
}
