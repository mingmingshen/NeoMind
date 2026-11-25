//! InputPipeline: authorized multi-source reads.

use std::sync::Arc;

use crate::error::Result;
use crate::model::{
    EvidenceDetail, EvidenceItem, EvidenceQuality, EvidenceReport, ExternalReadCounts,
    InputModality, InputRequirement, InputSourceRef, MissingPolicy,
};
use crate::ports::{ClockPort, SourceReadPort};

mod source_collector;
pub use source_collector::CountingSourcePort;

pub struct InputPipeline {
    sources: Arc<dyn SourceReadPort>,
    clock: Arc<dyn ClockPort>,
}

impl InputPipeline {
    pub fn new(sources: Arc<dyn SourceReadPort>, clock: Arc<dyn ClockPort>) -> Self {
        Self { sources, clock }
    }

    pub async fn collect(
        &self,
        requirements: &[InputRequirement],
    ) -> Result<(EvidenceReport, ExternalReadCounts)> {
        let mut items = Vec::new();
        let mut missing = Vec::new();
        let mut counts = ExternalReadCounts::default();

        for req in requirements {
            match &req.source {
                InputSourceRef::ExtensionInstance { id } => {
                    let op = req.operation.as_deref().unwrap_or("default");
                    bump_ext_count(&mut counts, id, op);
                    match self
                        .sources
                        .read_extension(id, op, &req.params, req.timeout_secs)
                        .await
                    {
                        Ok(r) => {
                            items.push(EvidenceItem {
                                key: req.key.clone(),
                                source_id: r.source_id,
                                result_type: req.result_type.clone(),
                                observed_at: r.observed_at,
                                valid_window: r.valid_window,
                                fetched_at: self.clock.now_ms(),
                                quality: EvidenceQuality::Fresh,
                                binding_proof: r.binding_proof,
                                detail: Some(detail_for(&req.result_type, r.value)),
                            });
                        }
                        Err(e) => {
                            missing.push(req.key.clone());
                            items.push(missing_item(req, e.to_string(), self.clock.now_ms()));
                            if matches!(req.on_missing, MissingPolicy::Skip) {
                                return Ok((
                                    EvidenceReport {
                                        items,
                                        missing_keys: missing,
                                        skip_reason: Some(format!("missing {}", req.key)),
                                    },
                                    counts,
                                ));
                            }
                        }
                    }
                }
                InputSourceRef::DeviceMetric { device_id, field } => {
                    counts.other += 1;
                    match self
                        .sources
                        .read_device_metric(device_id, field, &req.params, req.timeout_secs)
                        .await
                    {
                        Ok(r) => {
                            items.push(EvidenceItem {
                                key: req.key.clone(),
                                source_id: r.source_id,
                                result_type: req.result_type.clone(),
                                observed_at: r.observed_at,
                                valid_window: r.valid_window,
                                fetched_at: self.clock.now_ms(),
                                quality: EvidenceQuality::Fresh,
                                binding_proof: r.binding_proof,
                                detail: Some(detail_for(&req.result_type, r.value)),
                            });
                        }
                        Err(e) => {
                            missing.push(req.key.clone());
                            items.push(missing_item(req, e.to_string(), self.clock.now_ms()));
                        }
                    }
                }
                InputSourceRef::WholeDevice { device_id } => {
                    counts.other += 1;
                    match self
                        .sources
                        .read_device_metric(device_id, "*", &req.params, req.timeout_secs)
                        .await
                    {
                        Ok(r) => {
                            items.push(EvidenceItem {
                                key: req.key.clone(),
                                source_id: r.source_id,
                                result_type: req.result_type.clone(),
                                observed_at: r.observed_at,
                                valid_window: r.valid_window,
                                fetched_at: self.clock.now_ms(),
                                quality: EvidenceQuality::Fresh,
                                binding_proof: r.binding_proof,
                                detail: Some(detail_for(&req.result_type, r.value)),
                            });
                        }
                        Err(e) => {
                            missing.push(req.key.clone());
                            items.push(missing_item(req, e.to_string(), self.clock.now_ms()));
                        }
                    }
                }
                InputSourceRef::UpstreamAi { .. } | InputSourceRef::CallerPayload => {
                    // Not needed for T12; mark missing unless Ignore.
                    if !matches!(req.on_missing, MissingPolicy::Ignore) {
                        missing.push(req.key.clone());
                        items.push(missing_item(
                            req,
                            "source not wired in M1 harness".into(),
                            self.clock.now_ms(),
                        ));
                    }
                }
            }
        }

        Ok((
            EvidenceReport {
                items,
                missing_keys: missing,
                skip_reason: None,
            },
            counts,
        ))
    }
}

fn bump_ext_count(counts: &mut ExternalReadCounts, id: &str, op: &str) {
    let id_l = id.to_ascii_lowercase();
    let op_l = op.to_ascii_lowercase();
    if id_l.contains("weather") || op_l.contains("weather") || op_l == "current" && id_l.contains("weather") {
        counts.weather += 1;
    } else if id_l.contains("tariff")
        || id_l.contains("price")
        || op_l.contains("tou")
        || op_l.contains("tariff")
    {
        counts.tariff += 1;
    } else if id_l.contains("weather") {
        counts.weather += 1;
    } else {
        // Heuristic: weather extension ops named current → weather
        if op_l == "current" {
            counts.weather += 1;
        } else {
            counts.other += 1;
        }
    }
}

fn detail_for(modality: &InputModality, value: serde_json::Value) -> EvidenceDetail {
    match modality {
        InputModality::Text => EvidenceDetail::Text {
            text: value.as_str().unwrap_or(&value.to_string()).to_string(),
        },
        InputModality::TimeSeries => {
            let points = value
                .as_array()
                .map(|a| a.len())
                .or_else(|| value.get("points").and_then(|p| p.as_array()).map(|a| a.len()))
                .unwrap_or(0);
            EvidenceDetail::TimeSeries {
                points,
                value: Some(value),
            }
        }
        _ => EvidenceDetail::Structured { value },
    }
}

fn missing_item(req: &InputRequirement, reason: String, now: i64) -> EvidenceItem {
    let quality = match req.on_missing {
        MissingPolicy::ReportStale => EvidenceQuality::Stale,
        MissingPolicy::Ignore => EvidenceQuality::Missing,
        _ => EvidenceQuality::Error { reason: reason.clone() },
    };
    let source_id = match &req.source {
        InputSourceRef::ExtensionInstance { id } => format!("extension:{id}"),
        InputSourceRef::DeviceMetric { device_id, field } => {
            format!("device:{device_id}:{field}")
        }
        InputSourceRef::WholeDevice { device_id } => format!("device:{device_id}"),
        InputSourceRef::UpstreamAi { agent_id, field } => format!("ai:{agent_id}:{field}"),
        InputSourceRef::CallerPayload => "caller".into(),
    };
    EvidenceItem {
        key: req.key.clone(),
        source_id,
        result_type: req.result_type.clone(),
        observed_at: None,
        valid_window: None,
        fetched_at: now,
        quality,
        binding_proof: format!("missing:{reason}"),
        detail: None,
    }
}
