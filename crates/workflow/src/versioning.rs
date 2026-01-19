//! Workflow version management.
//!
//! This module provides functionality for versioning workflows,
//! including version history, rollback, and diff generation.

use crate::workflow::Workflow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Version identifier for workflows.
///
/// Uses semantic versioning (MAJOR.MINOR.PATCH).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkflowVersion {
    /// Major version - incompatible changes
    pub major: u32,
    /// Minor version - backwards-compatible functionality
    pub minor: u32,
    /// Patch version - backwards-compatible bug fixes
    pub patch: u32,
}

impl WorkflowVersion {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Create version 1.0.0.
    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }

    /// Increment major version.
    pub fn increment_major(&self) -> Self {
        Self::new(self.major + 1, 0, 0)
    }

    /// Increment minor version.
    pub fn increment_minor(&self) -> Self {
        Self::new(self.major, self.minor + 1, 0)
    }

    /// Increment patch version.
    pub fn increment_patch(&self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Parse from semver string.
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat(s.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| VersionError::InvalidFormat(s.to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| VersionError::InvalidFormat(s.to_string()))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| VersionError::InvalidFormat(s.to_string()))?;

        Ok(Self::new(major, minor, patch))
    }
}

impl fmt::Display for WorkflowVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for WorkflowVersion {
    fn default() -> Self {
        Self::initial()
    }
}

/// Type of version change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionChangeType {
    /// Major version change (breaking)
    Major,
    /// Minor version change (feature)
    Minor,
    /// Patch version change (bug fix)
    Patch,
}

/// Version error types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum VersionError {
    #[error("Invalid version format: {0}")]
    InvalidFormat(String),

    #[error("Version not found: {0}")]
    NotFound(String),

    #[error("Cannot rollback: {0}")]
    RollbackError(String),
}

/// A single workflow version snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    /// Version of this snapshot
    pub version: WorkflowVersion,
    /// The workflow definition at this version
    pub workflow: Workflow,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Who/what created this version
    pub created_by: String,
    /// Change description
    pub description: String,
    /// Type of change
    pub change_type: VersionChangeType,
}

impl WorkflowSnapshot {
    /// Create a new workflow snapshot.
    pub fn new(
        version: WorkflowVersion,
        workflow: Workflow,
        created_by: String,
        description: String,
        change_type: VersionChangeType,
    ) -> Self {
        Self {
            version,
            workflow,
            created_at: Utc::now(),
            created_by,
            description,
            change_type,
        }
    }
}

/// Diff between two workflow versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDiff {
    /// From version
    pub from_version: WorkflowVersion,
    /// To version
    pub to_version: WorkflowVersion,
    /// Changes detected
    pub changes: Vec<VersionChange>,
}

/// A single change in a workflow diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionChange {
    /// Type of change
    pub change_type: ChangeType,
    /// Path to the changed element (e.g., "steps.0.name")
    pub path: String,
    /// Old value (if applicable)
    pub old_value: Option<serde_json::Value>,
    /// New value (if applicable)
    pub new_value: Option<serde_json::Value>,
}

/// Types of changes in a workflow diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// A step was added
    StepAdded,
    /// A step was removed
    StepRemoved,
    /// A step was modified
    StepModified,
    /// A trigger was added
    TriggerAdded,
    /// A trigger was removed
    TriggerRemoved,
    /// A variable was added
    VariableAdded,
    /// A variable was removed
    VariableRemoved,
    /// A variable was modified
    VariableModified,
    /// Metadata changed (name, description, etc.)
    MetadataChanged,
}

/// Manager for workflow versions.
pub struct VersionManager {
    /// Storage for workflow version histories
    /// Key is workflow ID, value is vector of snapshots in version order
    histories: Arc<RwLock<HashMap<String, Vec<WorkflowSnapshot>>>>,
    /// Maximum versions to keep per workflow
    max_versions: usize,
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionManager {
    /// Create a new version manager.
    pub fn new() -> Self {
        Self {
            histories: Arc::new(RwLock::new(HashMap::new())),
            max_versions: 50, // Keep last 50 versions by default
        }
    }

    /// Create a version manager with custom max versions.
    pub fn with_max_versions(max_versions: usize) -> Self {
        let mut manager = Self::new();
        manager.max_versions = max_versions;
        manager
    }

    /// Create a new version of a workflow.
    pub async fn create_version(
        &self,
        workflow_id: &str,
        workflow: Workflow,
        created_by: String,
        description: String,
        change_type: VersionChangeType,
    ) -> Result<WorkflowVersion, VersionError> {
        let mut histories = self.histories.write().await;

        let history = histories.entry(workflow_id.to_string()).or_insert_with(Vec::new);

        // Determine next version
        let next_version = if let Some(last) = history.last() {
            match change_type {
                VersionChangeType::Major => last.version.increment_major(),
                VersionChangeType::Minor => last.version.increment_minor(),
                VersionChangeType::Patch => last.version.increment_patch(),
            }
        } else {
            WorkflowVersion::initial()
        };

        let snapshot = WorkflowSnapshot::new(
            next_version.clone(),
            workflow,
            created_by,
            description,
            change_type,
        );

        history.push(snapshot);

        // Prune old versions if exceeding limit
        if history.len() > self.max_versions {
            let remove_count = history.len() - self.max_versions;
            // Always keep the initial version
            if history.len() > remove_count + 1 {
                history.drain(1..remove_count + 1);
            }
        }

        Ok(next_version)
    }

    /// Get a specific version of a workflow.
    pub async fn get_version(
        &self,
        workflow_id: &str,
        version: &WorkflowVersion,
    ) -> Option<Workflow> {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .and_then(|history| history.iter().find(|s| &s.version == version))
            .map(|snapshot| snapshot.workflow.clone())
    }

    /// Get the latest version of a workflow.
    pub async fn get_latest(&self, workflow_id: &str) -> Option<Workflow> {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .and_then(|history| history.last())
            .map(|snapshot| snapshot.workflow.clone())
    }

    /// Get all versions of a workflow.
    pub async fn list_versions(&self, workflow_id: &str) -> Vec<WorkflowVersion> {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .map(|history| history.iter().map(|s| s.version.clone()).collect())
            .unwrap_or_default()
    }

    /// Get version history with metadata.
    pub async fn get_history(&self, workflow_id: &str) -> Vec<WorkflowSnapshot> {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Rollback a workflow to a previous version.
    ///
    /// Creates a new version with the rollback content.
    pub async fn rollback(
        &self,
        workflow_id: &str,
        to_version: &WorkflowVersion,
        created_by: String,
        reason: String,
    ) -> Result<Workflow, VersionError> {
        let target_workflow = self
            .get_version(workflow_id, to_version)
            .await
            .ok_or_else(|| VersionError::NotFound(to_version.to_string()))?;

        // Create a new version with the rollback
        let current_version = self
            .get_latest_version(workflow_id)
            .await
            .unwrap_or_else(WorkflowVersion::initial);

        // Determine the new version (increment patch for rollback)
        let new_version = current_version.increment_patch();

        let snapshot = WorkflowSnapshot::new(
            new_version,
            target_workflow.clone(),
            created_by,
            format!("Rollback to {}: {}", to_version, reason),
            VersionChangeType::Patch,
        );

        let mut histories = self.histories.write().await;
        let history = histories.entry(workflow_id.to_string()).or_insert_with(Vec::new);
        history.push(snapshot);

        Ok(target_workflow)
    }

    /// Get the latest version number for a workflow.
    async fn get_latest_version(&self, workflow_id: &str) -> Option<WorkflowVersion> {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .and_then(|history| history.last())
            .map(|snapshot| snapshot.version.clone())
    }

    /// Generate a diff between two versions.
    pub async fn diff(
        &self,
        workflow_id: &str,
        from_version: &WorkflowVersion,
        to_version: &WorkflowVersion,
    ) -> Option<WorkflowDiff> {
        let from_workflow = self.get_version(workflow_id, from_version).await?;
        let to_workflow = self.get_version(workflow_id, to_version).await?;

        let changes = self.compute_diff(&from_workflow, &to_workflow);

        Some(WorkflowDiff {
            from_version: from_version.clone(),
            to_version: to_version.clone(),
            changes,
        })
    }

    /// Compute differences between two workflows.
    fn compute_diff(&self, from: &Workflow, to: &Workflow) -> Vec<VersionChange> {
        let mut changes = Vec::new();

        // Check metadata changes
        if from.name != to.name {
            changes.push(VersionChange {
                change_type: ChangeType::MetadataChanged,
                path: "name".to_string(),
                old_value: Some(serde_json::json!(from.name)),
                new_value: Some(serde_json::json!(to.name)),
            });
        }

        if from.description != to.description {
            changes.push(VersionChange {
                change_type: ChangeType::MetadataChanged,
                path: "description".to_string(),
                old_value: Some(serde_json::json!(from.description)),
                new_value: Some(serde_json::json!(to.description)),
            });
        }

        // Check step changes
        let max_steps = from.steps.len().max(to.steps.len());
        for i in 0..max_steps {
            let path = format!("steps.{}", i);

            match (from.steps.get(i), to.steps.get(i)) {
                (None, Some(_)) => {
                    changes.push(VersionChange {
                        change_type: ChangeType::StepAdded,
                        path,
                        old_value: None,
                        new_value: None, // Would need to serialize step
                    });
                }
                (Some(_), None) => {
                    changes.push(VersionChange {
                        change_type: ChangeType::StepRemoved,
                        path,
                        old_value: None,
                        new_value: None,
                    });
                }
                (Some(from_step), Some(to_step)) => {
                    // Compare via JSON serialization since Step doesn't implement PartialEq
                    let from_json = serde_json::to_value(from_step);
                    let to_json = serde_json::to_value(to_step);
                    match (from_json, to_json) {
                        (Ok(f), Ok(t)) if f != t => {
                            changes.push(VersionChange {
                                change_type: ChangeType::StepModified,
                                path,
                                old_value: Some(f),
                                new_value: Some(t),
                            });
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // Check variable changes
        for (key, _) in from.variables.iter() {
            if !to.variables.contains_key(key) {
                changes.push(VersionChange {
                    change_type: ChangeType::VariableRemoved,
                    path: format!("variables.{}", key),
                    old_value: from.variables.get(key).cloned(),
                    new_value: None,
                });
            }
        }

        for (key, _) in to.variables.iter() {
            if !from.variables.contains_key(key) {
                changes.push(VersionChange {
                    change_type: ChangeType::VariableAdded,
                    path: format!("variables.{}", key),
                    old_value: None,
                    new_value: to.variables.get(key).cloned(),
                });
            }
        }

        changes
    }

    /// Delete all history for a workflow.
    pub async fn delete_history(&self, workflow_id: &str) {
        let mut histories = self.histories.write().await;
        histories.remove(workflow_id);
    }

    /// Get count of versions for a workflow.
    pub async fn version_count(&self, workflow_id: &str) -> usize {
        let histories = self.histories.read().await;
        histories
            .get(workflow_id)
            .map(|h| h.len())
            .unwrap_or(0)
    }

    /// Export version history as JSON.
    pub async fn export_history(&self, workflow_id: &str) -> Result<String, VersionError> {
        let history = self.get_history(workflow_id).await;
        serde_json::to_string_pretty(&history)
            .map_err(|e| VersionError::InvalidFormat(format!("Export failed: {}", e)))
    }

    /// Import version history from JSON.
    pub async fn import_history(
        &self,
        workflow_id: &str,
        json_data: &str,
    ) -> Result<(), VersionError> {
        let snapshots: Vec<WorkflowSnapshot> = serde_json::from_str(json_data)
            .map_err(|e| VersionError::InvalidFormat(format!("Import failed: {}", e)))?;

        let mut histories = self.histories.write().await;
        histories.insert(workflow_id.to_string(), snapshots);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{Step, Trigger, TriggerType};

    #[test]
    fn test_version_parsing() {
        let v = WorkflowVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        assert_eq!(v.increment_major(), WorkflowVersion::new(2, 0, 0));
        assert_eq!(v.increment_minor(), WorkflowVersion::new(1, 3, 0));
        assert_eq!(v.increment_patch(), WorkflowVersion::new(1, 2, 4));
    }

    #[test]
    fn test_version_display() {
        let v = WorkflowVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[tokio::test]
    async fn test_version_manager_create_version() {
        let manager = VersionManager::new();
        let workflow = Workflow::new("test", "Test Workflow");

        let v1 = manager
            .create_version(
                "test",
                workflow.clone(),
                "system".to_string(),
                "Initial version".to_string(),
                VersionChangeType::Major,
            )
            .await
            .unwrap();

        assert_eq!(v1, WorkflowVersion::initial());

        let v2 = manager
            .create_version(
                "test",
                workflow.clone(),
                "user".to_string(),
                "Minor update".to_string(),
                VersionChangeType::Minor,
            )
            .await
            .unwrap();

        assert_eq!(v2, WorkflowVersion::new(1, 1, 0));
    }

    #[tokio::test]
    async fn test_version_manager_rollback() {
        let manager = VersionManager::new();
        let workflow1 = Workflow::new("test", "Test Workflow v1");
        let workflow2 = Workflow::new("test", "Test Workflow v2");

        // Create initial version
        manager
            .create_version(
                "test",
                workflow1.clone(),
                "system".to_string(),
                "Initial".to_string(),
                VersionChangeType::Major,
            )
            .await
            .unwrap();

        // Create second version
        manager
            .create_version(
                "test",
                workflow2,
                "user".to_string(),
                "Update".to_string(),
                VersionChangeType::Patch,
            )
            .await
            .unwrap();

        // Rollback to first version
        let rolled_back = manager
            .rollback(
                "test",
                &WorkflowVersion::initial(),
                "admin".to_string(),
                "Mistake made".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(rolled_back.name, "Test Workflow v1");
    }

    #[tokio::test]
    async fn test_version_list() {
        let manager = VersionManager::new();
        let workflow = Workflow::new("test", "Test Workflow");

        manager
            .create_version(
                "test",
                workflow.clone(),
                "system".to_string(),
                "v1".to_string(),
                VersionChangeType::Major,
            )
            .await
            .unwrap();

        manager
            .create_version(
                "test",
                workflow.clone(),
                "system".to_string(),
                "v2".to_string(),
                VersionChangeType::Minor,
            )
            .await
            .unwrap();

        let versions = manager.list_versions("test").await;
        assert_eq!(versions.len(), 2);
    }
}
