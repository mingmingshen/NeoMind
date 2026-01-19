//! Zero-Config Auto-Onboarding for Unknown Devices
//!
//! This module handles the automatic discovery and onboarding of unknown devices:
//! 1. Collects data samples from unknown devices
//! 2. Uses AI to analyze samples and generate device types
//! 3. Creates draft devices for user review
//! 4. Auto-registers high-confidence devices

use crate::discovery::types::*;
use crate::discovery::{DataPathExtractor, SemanticInference, VirtualMetricGenerator};
use edge_ai_core::{EventBus, LlmRuntime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type signature for matching similar device types
///
/// A type signature is a fingerprint based on the semantic and data types
/// of metrics, allowing devices with the same data structure to share types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeSignature {
    /// Ordered list of metric signatures (semantic_type, data_type pairs)
    pub metric_signatures: Vec<(String, String)>,
    /// Device category for coarse filtering
    pub category: String,
}

impl TypeSignature {
    /// Compute a hash from the signature
    pub fn to_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.metric_signatures.hash(&mut hasher);
        self.category.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Auto-onboarding manager for zero-config device discovery
pub struct AutoOnboardManager {
    /// LLM runtime for AI analysis
    llm: Arc<dyn LlmRuntime>,
    /// Event bus for publishing events
    event_bus: Arc<EventBus>,
    /// Configuration
    config: AutoOnboardConfig,
    /// Draft devices being tracked
    drafts: Arc<RwLock<HashMap<String, DraftDevice>>>,
    /// Type signatures mapped to device_type IDs
    /// signature_hash -> device_type
    type_signatures: Arc<RwLock<HashMap<String, String>>>,
    /// Reverse mapping: device_type -> signature_hash
    device_type_signatures: Arc<RwLock<HashMap<String, String>>>,
    /// Path extractor for analyzing samples
    path_extractor: DataPathExtractor,
    /// Semantic inference
    semantic_inference: SemanticInference,
    /// Virtual metric generator
    metric_generator: VirtualMetricGenerator,
}

impl AutoOnboardManager {
    /// Create a new auto-onboard manager
    pub fn new(llm: Arc<dyn LlmRuntime>, event_bus: Arc<EventBus>) -> Self {
        let config = AutoOnboardConfig::default();
        let path_extractor = DataPathExtractor::new(llm.clone());
        let semantic_inference = SemanticInference::new(llm.clone());
        let metric_generator = VirtualMetricGenerator::new(llm.clone());

        Self {
            llm,
            event_bus,
            config,
            drafts: Arc::new(RwLock::new(HashMap::new())),
            type_signatures: Arc::new(RwLock::new(HashMap::new())),
            device_type_signatures: Arc::new(RwLock::new(HashMap::new())),
            path_extractor,
            semantic_inference,
            metric_generator,
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: AutoOnboardConfig) -> Self {
        self.config = config;
        self
    }

    /// Process incoming data from an unknown device
    ///
    /// Returns whether the data was accepted for collection
    pub async fn process_unknown_device(
        &self,
        device_id: &str,
        source: &str,
        data: &serde_json::Value,
    ) -> Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        let drafts = self.drafts.read().await;

        // Check if there's an existing draft for this device
        if let Some(draft) = drafts.get(device_id) {
            drop(drafts);
            return self.add_sample_to_draft(device_id, data).await;
        }

        // Check if we're at capacity
        if drafts.len() >= self.config.max_draft_devices {
            return Ok(false);
        }
        drop(drafts);

        // Create new draft device
        self.create_draft(device_id, source, data).await
    }

    /// Create a new draft device
    async fn create_draft(
        &self,
        device_id: &str,
        source: &str,
        data: &serde_json::Value,
    ) -> Result<bool> {
        let mut drafts = self.drafts.write().await;

        let mut draft = DraftDevice::new(
            device_id.to_string(),
            source.to_string(),
            self.config.max_samples,
        );

        let sample = DeviceSample::from_json(data.clone(), source);
        draft.add_sample(sample);

        drafts.insert(device_id.to_string(), draft.clone());

        // Publish event
        self.publish_event(AutoOnboardEvent::DraftCreated {
            draft_id: draft.id.clone(),
            device_id: device_id.to_string(),
            source: source.to_string(),
        });

        tracing::info!(
            "Created draft device '{}' from source {} (1/{} samples)",
            device_id,
            source,
            self.config.max_samples
        );

        Ok(true)
    }

    /// Add a sample to an existing draft
    async fn add_sample_to_draft(
        &self,
        device_id: &str,
        data: &serde_json::Value,
    ) -> Result<bool> {
        let mut drafts = self.drafts.write().await;
        let draft = drafts.get_mut(device_id).ok_or_else(|| {
            DiscoveryError::InvalidData(format!("Draft device not found: {}", device_id))
        })?;

        // Don't add samples if already analyzing
        if draft.status != DraftDeviceStatus::Collecting {
            return Ok(false);
        }

        let source = draft.source.clone();
        let added = draft.add_sample(DeviceSample::from_json(data.clone(), &source));

        if added {
            let count = draft.samples.len();
            tracing::debug!(
                "Added sample to draft '{}' ({}/{} collected)",
                device_id,
                count,
                draft.max_samples
            );

            self.publish_event(AutoOnboardEvent::SampleCollected {
                draft_id: draft.id.clone(),
                sample_count: count,
            });

            // Check if ready for analysis
            if draft.ready_for_analysis(self.config.min_samples) {
                let draft_id = draft.id.clone();
                let device_id = draft.device_id.clone();
                let samples = draft.json_samples();
                drop(drafts);

                // Trigger analysis in background
                let manager = self.clone_for_task();
                tokio::spawn(async move {
                    let _ = manager.analyze_device(&draft_id, &device_id, samples).await;
                });
            }
        }

        Ok(added)
    }

    /// Analyze collected samples and generate device type
    pub async fn analyze_device(
        &self,
        draft_id: &str,
        device_id: &str,
        samples: Vec<serde_json::Value>,
    ) -> Result<GeneratedDeviceType> {
        // Update status to Analyzing
        {
            let mut drafts = self.drafts.write().await;
            if let Some(draft) = drafts.get_mut(device_id) {
                draft.set_status(DraftDeviceStatus::Analyzing);
            }
        }

        self.publish_event(AutoOnboardEvent::AnalysisStarted {
            draft_id: draft_id.to_string(),
            sample_count: samples.len(),
        });

        tracing::info!(
            "Analyzing draft device '{}' with {} samples",
            device_id,
            samples.len()
        );

        // Convert samples to DeviceSample format
        let device_samples: Vec<DeviceSample> = samples
            .into_iter()
            .enumerate()
            .map(|(i, v)| DeviceSample {
                raw_data: serde_json::to_vec(&v).unwrap_or_default(),
                parsed: Some(v),
                source: format!("sample_{}", i),
                timestamp: chrono::Utc::now().timestamp(),
            })
            .collect();

        // Use VirtualMetricGenerator to analyze
        let context = InferenceContext::default();
        let metrics = self
            .metric_generator
            .generate_metrics(device_id, &device_samples, &context)
            .await
            .map_err(|e| DiscoveryError::Llm(e.to_string()))?;

        // Infer device category from metrics
        let category = self.infer_category(&metrics);

        // Check for existing type with matching signature
        let existing_type = self.find_matching_type(&metrics, &category).await;

        let (type_id, display_name, reusing_type) = if let Some(existing_type_id) = existing_type {
            tracing::info!(
                "Found matching type signature for device '{}': reusing type '{}'",
                device_id,
                existing_type_id
            );
            (existing_type_id.clone(), format!("{} (reused)", existing_type_id), true)
        } else {
            // Generate new device type ID
            let new_type_id = format!("auto_{}", device_id.replace('-', "_"));
            let new_display_name = self.generate_device_name(device_id, &category, &metrics);
            (new_type_id, new_display_name, false)
        };

        // Generate MDL definition
        let mdl_definition = self.generate_mdl(&type_id, &display_name, &metrics, &category).await?;

        let generated_type = GeneratedDeviceType::from_discovered(
            type_id.clone(),
            display_name,
            metrics.clone(),
            vec![], // Commands - TODO: infer from data patterns
            category.clone(),
            mdl_definition,
            device_samples.len(),
        );

        let confidence = generated_type.confidence;
        let type_id_for_event = type_id.clone(); // Clone for event publishing

        // Register type signature if this is a new type
        if !reusing_type {
            self.register_type_signature(&metrics, &category, &type_id).await;
        }

        // Update draft with generated type
        {
            let mut drafts = self.drafts.write().await;
            if let Some(draft) = drafts.get_mut(device_id) {
                draft.generated_type = Some(generated_type.clone());

                // Decide next status based on confidence
                // Reused types get higher confidence since they're proven
                let effective_confidence = if reusing_type {
                    (confidence + 0.1).min(1.0) // Boost confidence for reused types
                } else {
                    confidence
                };

                if effective_confidence >= self.config.auto_approve_threshold || draft.auto_approve {
                    draft.set_status(DraftDeviceStatus::Registering);
                    // Will trigger registration in another task
                } else {
                    draft.set_status(DraftDeviceStatus::PendingReview);
                }
            }
        }

        self.publish_event(AutoOnboardEvent::AnalysisCompleted {
            draft_id: draft_id.to_string(),
            device_type: type_id_for_event,
            confidence,
        });

        tracing::info!(
            "Analysis complete for '{}': type={}, confidence={}, reusing={}",
            device_id,
            type_id,
            confidence,
            reusing_type
        );

        // Auto-register if high confidence
        let effective_confidence = if reusing_type {
            (confidence + 0.1).min(1.0)
        } else {
            confidence
        };

        if effective_confidence >= self.config.auto_approve_threshold {
            let manager = self.clone_for_task();
            let draft_id = draft_id.to_string();
            let device_id_owned = device_id.to_string();
            tokio::spawn(async move {
                let _ = manager.register_device(&draft_id, &device_id_owned).await;
            });
        }

        // Get the generated type from drafts
        let drafts = self.drafts.read().await;
        if let Some(draft) = drafts.get(device_id) {
            if let Some(ref gen_type) = draft.generated_type {
                return Ok(gen_type.clone());
            }
        }

        Err(DiscoveryError::Parse("Generated type not found".to_string()))
    }

    /// Register a device from its draft
    pub async fn register_device(
        &self,
        draft_id: &str,
        device_id: &str,
    ) -> Result<()> {
        let (device_type, mdl_def) = {
            let drafts = self.drafts.read().await;
            let draft = drafts.get(device_id).ok_or_else(|| {
                DiscoveryError::InvalidData(format!("Draft not found: {}", device_id))
            })?;

            let gen_type = draft.generated_type.as_ref().ok_or_else(|| {
                DiscoveryError::InvalidData("No generated type for draft".to_string())
            })?;

            (gen_type.device_type.clone(), gen_type.mdl_definition.clone())
        };

        tracing::info!(
            "Registering device '{}' with type '{}'",
            device_id,
            device_type
        );

        // TODO: Actually register with MDL registry and device registry
        // This would call:
        // 1. mdl_registry.register(mdl_def)
        // 2. device_registry.add(device_id, device_type)

        // Update draft status
        {
            let mut drafts = self.drafts.write().await;
            if let Some(draft) = drafts.get_mut(device_id) {
                draft.set_status(DraftDeviceStatus::Registered);
            }
        }

        self.publish_event(AutoOnboardEvent::DeviceRegistered {
            draft_id: draft_id.to_string(),
            device_id: device_id.to_string(),
            device_type,
        });

        tracing::info!("Device '{}' successfully registered", device_id);

        Ok(())
    }

    /// Reject a draft device
    pub async fn reject_device(&self, device_id: &str, reason: &str) -> Result<()> {
        let mut drafts = self.drafts.write().await;
        let draft = drafts.get_mut(device_id).ok_or_else(|| {
            DiscoveryError::InvalidData(format!("Draft not found: {}", device_id))
        })?;

        draft.set_status(DraftDeviceStatus::Rejected);
        draft.error_message = Some(reason.to_string());

        self.publish_event(AutoOnboardEvent::DeviceRejected {
            draft_id: draft.id.clone(),
            reason: reason.to_string(),
        });

        tracing::info!("Draft device '{}' rejected: {}", device_id, reason);

        Ok(())
    }

    /// Get all draft devices
    pub async fn get_drafts(&self) -> Vec<DraftDevice> {
        let drafts = self.drafts.read().await;
        drafts.values().cloned().collect()
    }

    /// Get a specific draft device
    pub async fn get_draft(&self, device_id: &str) -> Option<DraftDevice> {
        let drafts = self.drafts.read().await;
        drafts.get(device_id).cloned()
    }

    /// Update draft device (user edits)
    pub async fn update_draft(
        &self,
        device_id: &str,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<()> {
        let mut drafts = self.drafts.write().await;
        let draft = drafts.get_mut(device_id).ok_or_else(|| {
            DiscoveryError::InvalidData(format!("Draft not found: {}", device_id))
        })?;

        if let Some(n) = name {
            draft.user_name = Some(n);
        }
        if let Some(d) = description {
            draft.user_description = Some(d);
        }
        draft.updated_at = chrono::Utc::now().timestamp();

        Ok(())
    }

    /// Clean up old draft devices
    pub async fn cleanup_old_drafts(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut drafts = self.drafts.write().await;

        let to_remove: Vec<String> = drafts
            .iter()
            .filter(|(_, d)| {
                now - d.updated_at > self.config.draft_retention_secs as i64
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            drafts.remove(id);
        }

        to_remove.len()
    }

    /// Infer device category from metrics
    fn infer_category(&self, metrics: &[DiscoveredMetric]) -> DeviceCategory {
        use SemanticType::*;

        let has_temp = metrics.iter().any(|m| m.semantic_type == Temperature);
        let has_humid = metrics.iter().any(|m| m.semantic_type == Humidity);
        let has_motion = metrics.iter().any(|m| m.semantic_type == Motion);
        let has_light = metrics.iter().any(|m| m.semantic_type == Light);
        let has_switch = metrics.iter().any(|m| m.semantic_type == Switch);
        let has_power = metrics.iter().any(|m| m.semantic_type == Power || m.semantic_type == Energy);
        let has_battery = metrics.iter().any(|m| m.semantic_type == Battery);

        // Detection-related patterns
        let has_detections = metrics.iter().any(|m| m.path.contains("detection") || m.path.contains("object"));
        let has_image = metrics.iter().any(|m| m.path.contains("image") || m.path.contains("frame"));

        match () {
            _ if has_image && has_detections => DeviceCategory::Camera,
            _ if has_temp && has_humid => DeviceCategory::MultiSensor,
            _ if has_temp => DeviceCategory::TemperatureSensor,
            _ if has_humid => DeviceCategory::HumiditySensor,
            _ if has_motion => DeviceCategory::MotionSensor,
            _ if has_light => DeviceCategory::LightSensor,
            _ if has_switch => DeviceCategory::Switch,
            _ if has_power => DeviceCategory::EnergyMonitor,
            _ if has_battery => DeviceCategory::Unknown, // Generic sensor
            _ => DeviceCategory::Unknown,
        }
    }

    /// Generate a display name for the device
    fn generate_device_name(
        &self,
        device_id: &str,
        category: &DeviceCategory,
        metrics: &[DiscoveredMetric],
    ) -> String {
        // Try to extract a meaningful name from device_id
        let base_name = device_id
            .split(['-', '_'])
            .next()
            .unwrap_or("device")
            .to_string();

        format!("{} {}", category.display_name(), base_name)
    }

    /// Generate MDL definition
    async fn generate_mdl(
        &self,
        type_id: &str,
        name: &str,
        metrics: &[DiscoveredMetric],
        category: &DeviceCategory,
    ) -> Result<serde_json::Value> {
        use serde_json::json;

        // Build metrics definition
        let metrics_def: Vec<serde_json::Value> = metrics
            .iter()
            .map(|m| {
                json!({
                    "name": m.name,
                    "path": m.path,
                    "data_type": m.data_type.display_name(),
                    "unit": m.unit,
                    "display_name": m.display_name,
                    "description": m.description,
                })
            })
            .collect();

        Ok(json!({
            "device_type": type_id,
            "name": name,
            "description": format!("Auto-generated {} definition", category.display_name()),
            "category": category.display_name(),
            "version": "1.0.0",
            "metrics": metrics_def,
            "commands": [],
            "generated_by": "neoalk-auto-onboard",
            "generated_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Publish an event
    fn publish_event(&self, event: AutoOnboardEvent) {
        if let Ok(event_json) = serde_json::to_value(&event) {
            // Publish to event bus
            // For now, just log
            tracing::debug!("Auto-onboard event: {:?}", event);
        }
    }

    /// Compute type signature from discovered metrics
    ///
    /// The signature is based on the ordered list of (semantic_type, data_type) pairs
    /// and the device category. This allows devices with the same data structure
    /// to be identified as the same type.
    pub fn compute_type_signature(&self, metrics: &[DiscoveredMetric], category: &DeviceCategory) -> TypeSignature {
        // Sort metrics by semantic type to ensure consistent ordering
        let mut sorted_metrics = metrics.to_vec();
        sorted_metrics.sort_by(|a, b| {
            // Sort by string representations since semantic_type and data_type don't implement Ord
            let a_semantic = format!("{:?}", a.semantic_type);
            let b_semantic = format!("{:?}", b.semantic_type);
            match a_semantic.cmp(&b_semantic) {
                std::cmp::Ordering::Equal => {
                    let a_data = format!("{:?}", a.data_type);
                    let b_data = format!("{:?}", b.data_type);
                    a_data.cmp(&b_data)
                }
                other => other,
            }
        });

        let metric_signatures = sorted_metrics
            .iter()
            .map(|m| {
                // Convert semantic_type and data_type to strings for consistent hashing
                let semantic_str = format!("{:?}", m.semantic_type);
                let data_type_str = format!("{:?}", m.data_type);
                (semantic_str, data_type_str)
            })
            .collect();

        TypeSignature {
            metric_signatures,
            category: format!("{:?}", category),
        }
    }

    /// Find an existing device type that matches the given metrics
    ///
    /// Returns None if no matching type exists, or Some(device_type_id) if found.
    pub async fn find_matching_type(&self, metrics: &[DiscoveredMetric], category: &DeviceCategory) -> Option<String> {
        let signature = self.compute_type_signature(metrics, category);
        let signature_hash = signature.to_hash();

        let signatures = self.type_signatures.read().await;
        signatures.get(&signature_hash).cloned()
    }

    /// Register a new type signature mapping
    ///
    /// Stores the relationship between a signature hash and device_type_id.
    pub async fn register_type_signature(&self, metrics: &[DiscoveredMetric], category: &DeviceCategory, device_type: &str) {
        let signature = self.compute_type_signature(metrics, category);
        let signature_hash = signature.to_hash();

        // Store signature -> device_type mapping
        {
            let mut signatures = self.type_signatures.write().await;
            signatures.insert(signature_hash.clone(), device_type.to_string());
        }

        // Store device_type -> signature mapping (for reverse lookup)
        {
            let mut reverse = self.device_type_signatures.write().await;
            reverse.insert(device_type.to_string(), signature_hash.clone());
        }

        tracing::debug!(
            "Registered type signature: {} -> {}",
            signature_hash,
            device_type
        );
    }

    /// Get all registered type signatures
    ///
    /// Returns a map of signature_hash -> device_type_id
    pub async fn get_all_type_signatures(&self) -> HashMap<String, String> {
        let signatures = self.type_signatures.read().await;
        signatures.clone()
    }

    /// Get the signature hash for a specific device type
    pub async fn get_type_signature(&self, device_type: &str) -> Option<String> {
        let reverse = self.device_type_signatures.read().await;
        reverse.get(device_type).cloned()
    }

    /// Rename a device type in the signature registry
    ///
    /// Updates the signature mappings when a device type is renamed.
    pub async fn rename_type_signature(&self, old_name: &str, new_name: &str) -> Result<()> {
        let signature_hash = {
            let reverse = self.device_type_signatures.read().await;
            reverse.get(old_name).cloned().ok_or_else(|| {
                DiscoveryError::InvalidData(format!("Device type not found in signatures: {}", old_name))
            })?
        };

        // Update signature -> device_type mapping
        {
            let mut signatures = self.type_signatures.write().await;
            if let Some(existing_type) = signatures.get(&signature_hash) {
                if existing_type == old_name {
                    signatures.insert(signature_hash.clone(), new_name.to_string());
                }
            }
        }

        // Update device_type -> signature mapping
        {
            let mut reverse = self.device_type_signatures.write().await;
            reverse.remove(old_name);
            reverse.insert(new_name.to_string(), signature_hash);
        }

        tracing::info!("Renamed type signature: {} -> {}", old_name, new_name);
        Ok(())
    }

    /// Remove a device type from the signature registry
    pub async fn remove_type_signature(&self, device_type: &str) -> Result<()> {
        let signature_hash = {
            let reverse = self.device_type_signatures.read().await;
            reverse.get(device_type).cloned().ok_or_else(|| {
                DiscoveryError::InvalidData(format!("Device type not found in signatures: {}", device_type))
            })?
        };

        // Remove from signature -> device_type mapping
        {
            let mut signatures = self.type_signatures.write().await;
            signatures.remove(&signature_hash);
        }

        // Remove from device_type -> signature mapping
        {
            let mut reverse = self.device_type_signatures.write().await;
            reverse.remove(device_type);
        }

        tracing::info!("Removed type signature for: {}", device_type);
        Ok(())
    }

    /// Check if a device type exists for the given signature
    pub async fn has_matching_type(&self, metrics: &[DiscoveredMetric], category: &DeviceCategory) -> bool {
        self.find_matching_type(metrics, category).await.is_some()
    }

    /// Clone for task spawning
    fn clone_for_task(&self) -> Self {
        Self {
            llm: self.llm.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
            drafts: self.drafts.clone(),
            type_signatures: self.type_signatures.clone(),
            device_type_signatures: self.device_type_signatures.clone(),
            path_extractor: DataPathExtractor::new(self.llm.clone()),
            semantic_inference: SemanticInference::new(self.llm.clone()),
            metric_generator: VirtualMetricGenerator::new(self.llm.clone()),
        }
    }
}

/// Clone implementation for task spawning
impl Clone for AutoOnboardManager {
    fn clone(&self) -> Self {
        self.clone_for_task()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_device_creation() {
        let draft = DraftDevice::new("test-device".to_string(), "mqtt".to_string(), 5);

        assert_eq!(draft.device_id, "test-device");
        assert_eq!(draft.source, "mqtt");
        assert_eq!(draft.status, DraftDeviceStatus::Collecting);
        assert_eq!(draft.samples.len(), 0);
        assert_eq!(draft.max_samples, 5);
    }

    #[test]
    fn test_add_sample() {
        let mut draft = DraftDevice::new("test-device".to_string(), "mqtt".to_string(), 3);
        let sample = DeviceSample::from_json(serde_json::json!({"temp": 25.5}), "test");

        assert!(draft.add_sample(sample));
        assert_eq!(draft.samples.len(), 1);
        assert!(draft.ready_for_analysis(1));
    }

    #[test]
    fn test_max_samples() {
        let mut draft = DraftDevice::new("test-device".to_string(), "mqtt".to_string(), 2);

        assert!(draft.add_sample(DeviceSample::from_json(
            serde_json::json!({"temp": 25.5}),
            "test",
        )));
        assert!(draft.add_sample(DeviceSample::from_json(
            serde_json::json!({"temp": 26.0}),
            "test",
        )));
        assert!(!draft.add_sample(DeviceSample::from_json(
            serde_json::json!({"temp": 26.5}),
            "test",
        )));

        assert_eq!(draft.samples.len(), 2);
    }

    #[test]
    fn test_status_transitions() {
        let mut draft = DraftDevice::new("test-device".to_string(), "mqtt".to_string(), 3);

        assert_eq!(draft.status, DraftDeviceStatus::Collecting);

        draft.set_status(DraftDeviceStatus::Analyzing);
        assert_eq!(draft.status, DraftDeviceStatus::Analyzing);

        draft.set_status(DraftDeviceStatus::PendingReview);
        assert_eq!(draft.status, DraftDeviceStatus::PendingReview);

        draft.set_status(DraftDeviceStatus::Registered);
        assert_eq!(draft.status, DraftDeviceStatus::Registered);
    }

    #[test]
    fn test_category_inference() {
        let manager = create_test_manager();

        // Temperature + Humidity = MultiSensor
        let metrics = vec![
            DiscoveredMetric {
                semantic_type: SemanticType::Temperature,
                ..Default::default()
            },
            DiscoveredMetric {
                semantic_type: SemanticType::Humidity,
                ..Default::default()
            },
        ];
        assert_eq!(
            manager.infer_category(&metrics),
            DeviceCategory::MultiSensor
        );

        // Only temperature = TemperatureSensor
        let metrics = vec![DiscoveredMetric {
            semantic_type: SemanticType::Temperature,
            ..Default::default()
        }];
        assert_eq!(
            manager.infer_category(&metrics),
            DeviceCategory::TemperatureSensor
        );
    }

    fn create_test_manager() -> AutoOnboardManager {
        use edge_ai_core::{EventBus, LlmRuntime};

        // Create a simple test helper - the manager only needs the reference
        // for infer_category which is a pure function
        struct DummyLlm;
        #[async_trait::async_trait]
        impl LlmRuntime for DummyLlm {
            fn backend_id(&self) -> edge_ai_core::llm::backend::BackendId {
                edge_ai_core::llm::backend::BackendId::Ollama
            }
            fn model_name(&self) -> &str {
                "dummy"
            }
            fn capabilities(&self) -> edge_ai_core::llm::backend::BackendCapabilities {
                edge_ai_core::llm::backend::BackendCapabilities::default()
            }
            fn generate(&self, _input: &edge_ai_core::llm::backend::LlmInput) -> edge_ai_core::llm::backend::LlmOutput {
                edge_ai_core::llm::backend::LlmOutput {
                    text: String::new(),
                    finish_reason: edge_ai_core::llm::backend::FinishReason::Stop,
                    usage: edge_ai_core::llm::backend::TokenUsage::default(),
                }
            }
            fn generate_stream(
                &self,
                _input: &edge_ai_core::llm::backend::LlmInput,
            ) -> edge_ai_core::futures::stream::BoxStream<
                'static,
                Result<(String, bool), edge_ai_core::llm::backend::LlmError>,
            > {
                Box::pin(edge_ai_core::futures::stream::empty())
            }
        }

        let llm = Arc::new(DummyLlm) as Arc<dyn LlmRuntime>;
        let event_bus = Arc::new(EventBus::new(100));

        AutoOnboardManager::new(llm, event_bus)
    }
}
