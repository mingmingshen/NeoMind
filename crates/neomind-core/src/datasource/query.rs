//! Unified Query Service
//!
//! This module provides a single query interface for all data source types.
//! It abstracts the underlying storage and provides a consistent API for
//! querying data from devices, extensions, and transforms.

use super::super::extension::executor::UnifiedStorage;
use super::{AggregatedValue, DataPoint, DataSourceId, DataSourceInfo, QueryParams, QueryResult};

/// Unified query service for all data sources
pub struct UnifiedQueryService {
    storage: std::sync::Arc<dyn UnifiedStorage + Send + Sync>,
}

impl UnifiedQueryService {
    pub fn new(storage: std::sync::Arc<dyn UnifiedStorage + Send + Sync>) -> Self {
        Self { storage }
    }

    /// Query data points from a data source
    pub async fn query(
        &self,
        source_id: &DataSourceId,
        params: &QueryParams,
    ) -> Result<QueryResult, QueryError> {
        let mut datapoints = self
            .storage
            .query_datapoints(source_id, params.start, params.end)
            .await
            .map_err(|e| QueryError::Storage(e.to_string()))?;

        // Apply limit if specified
        if let Some(limit) = params.limit {
            datapoints.truncate(limit);
        }

        // Compute aggregation if requested
        let aggregation = if let Some(agg_func) = &params.aggregation {
            Self::compute_aggregation(&datapoints, agg_func).map(|value| AggregatedValue {
                value,
                count: datapoints.len(),
                func: agg_func.clone(),
            })
        } else {
            None
        };

        Ok(QueryResult {
            source_id: source_id.clone(),
            datapoints,
            aggregation,
        })
    }

    /// Compute aggregation on datapoints
    fn compute_aggregation(datapoints: &[DataPoint], agg_func: &super::AggFunc) -> Option<f64> {
        if datapoints.is_empty() {
            return None;
        }

        let values: Vec<f64> = datapoints
            .iter()
            .filter_map(|dp| match &dp.value {
                crate::event::MetricValue::Float(v) => Some(*v),
                crate::event::MetricValue::Integer(v) => Some(*v as f64),
                _ => None,
            })
            .collect();

        if values.is_empty() {
            return None;
        }

        let result = match agg_func {
            super::AggFunc::Avg => values.iter().sum::<f64>() / values.len() as f64,
            super::AggFunc::Sum => values.iter().sum::<f64>(),
            super::AggFunc::Min => values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            super::AggFunc::Max => values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            super::AggFunc::Count => values.len() as f64,
            super::AggFunc::Last => *values.last().unwrap_or(&0.0),
        };

        Some(result)
    }

    /// Query latest data point
    pub async fn query_latest(
        &self,
        source_id: &DataSourceId,
        since: i64,
    ) -> Result<Option<DataPoint>, QueryError> {
        self.storage
            .query_latest(source_id, since)
            .await
            .map_err(|e| QueryError::Storage(e.to_string()))
    }

    /// Get all available data sources
    pub async fn list_sources(&self) -> Result<Vec<DataSourceInfo>, QueryError> {
        // This would be implemented by scanning the storage for all known sources
        // For now, return an empty list
        Ok(Vec::new())
    }

    /// Get data sources by type
    pub async fn list_sources_by_type(
        &self,
        _source_type: &super::DataSourceType,
    ) -> Result<Vec<DataSourceInfo>, QueryError> {
        // This would be implemented by scanning the storage
        Ok(Vec::new())
    }
}

/// Query errors
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Invalid data source: {0}")]
    InvalidSource(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Invalid query parameters: {0}")]
    InvalidParameters(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasource::{DataSourceId, DataSourceType};
    use crate::event::MetricValue;
    use crate::extension::executor::MemoryStorage;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_unified_query_service() {
        let storage: Arc<dyn UnifiedStorage + Send + Sync> = Arc::new(MemoryStorage::new());
        let service = UnifiedQueryService::new(storage.clone());

        // Write test data
        let source_id = DataSourceId {
            source_type: DataSourceType::Extension,
            source_id: "test-ext".to_string(),
            field_path: "test.value".to_string(),
        };

        let datapoint = DataPoint::new(12345, MetricValue::Float(42.0));
        storage
            .write_datapoint(&source_id, datapoint.clone())
            .await
            .unwrap();

        // Query the data
        let params = QueryParams::new(0, 99999);
        let result = service.query(&source_id, &params).await.unwrap();

        assert_eq!(result.datapoints.len(), 1);
        assert_eq!(result.datapoints[0].timestamp, 12345);
    }

    #[tokio::test]
    async fn test_query_latest() {
        let storage: Arc<dyn UnifiedStorage + Send + Sync> = Arc::new(MemoryStorage::new());
        let service = UnifiedQueryService::new(storage.clone());

        let source_id = DataSourceId {
            source_type: DataSourceType::Extension,
            source_id: "test-ext".to_string(),
            field_path: "test.value".to_string(),
        };

        // No data yet
        let result = service.query_latest(&source_id, 0).await.unwrap();
        assert!(result.is_none());

        // Add data
        let datapoint = DataPoint::new(12345, MetricValue::Float(42.0));
        storage
            .write_datapoint(&source_id, datapoint)
            .await
            .unwrap();

        // Query again
        let result = service.query_latest(&source_id, 0).await.unwrap();
        assert!(result.is_some());
    }
}
