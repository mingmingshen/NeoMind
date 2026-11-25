//! InputPipeline: authorized multi-source reads.

use std::sync::Arc;

use crate::error::Result;
use crate::model::{
    EvidenceDetail, EvidenceItem, EvidenceQuality, EvidenceReport, ExternalReadCounts,
    FreshnessPolicy, InputModality, InputRequirement, InputSourceRef, MissingPolicy,
};
use crate::ports::{ClockPort, SourceReadPort, SourceReadResult};

mod media_extractor;
mod report_snapshot;
mod source_collector;

pub use media_extractor::{
    classify_image_value, fixture_image_with_report_id, is_report_id_metric, ImageValue,
    MIN_BASE64_LEN,
};
pub use report_snapshot::{
    anchor_from_params, bind_image_to_report, image_point_belongs_to_report_id,
    image_record_id_equals, FrameBinding, ReportMetricPoint, SameReportAnchor,
};
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
                            items.push(fresh_item(req, r, self.clock.now_ms(), None));
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
                            let (item, is_missing) =
                                evidence_from_device_read(req, field, r, self.clock.now_ms());
                            if is_missing {
                                missing.push(req.key.clone());
                            }
                            items.push(item);
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
                            let field = req
                                .params
                                .get("image_field")
                                .and_then(|v| v.as_str())
                                .unwrap_or("image_data");
                            let (item, is_missing) =
                                evidence_from_device_read(req, field, r, self.clock.now_ms());
                            if is_missing {
                                missing.push(req.key.clone());
                            }
                            items.push(item);
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

/// Build evidence for a device read, applying SameReport ownership when configured.
fn evidence_from_device_read(
    req: &InputRequirement,
    field: &str,
    r: SourceReadResult,
    now: i64,
) -> (EvidenceItem, bool) {
    if matches!(req.freshness, FreshnessPolicy::SameReport)
        && matches!(req.result_type, InputModality::Image)
    {
        return same_report_image_item(req, field, r, now);
    }
    (fresh_item(req, r, now, Some(field)), false)
}

fn same_report_image_item(
    req: &InputRequirement,
    field: &str,
    r: SourceReadResult,
    now: i64,
) -> (EvidenceItem, bool) {
    let Some(anchor) = anchor_from_params(&req.params) else {
        return (
            unconfirmed_item(
                req,
                &r,
                now,
                "SameReport requires params.report_id from the triggering event".into(),
            ),
            true,
        );
    };

    let image_meta = r.value.get("metadata").cloned().or_else(|| {
        // Harness may stash metadata beside the value under `_metadata`.
        r.value.get("_metadata").cloned()
    });
    // Use the image's own observed timestamp — never force it to the anchor
    // (SameReport requires exact second equality when anchor_ts is set).
    let image = ReportMetricPoint {
        name: field.to_string(),
        timestamp: r.observed_at.unwrap_or(now),
        value: r.value.clone(),
        metadata: image_meta,
    };

    // Optional sibling points for `_raw` envelope checks (fixture array).
    let siblings = sibling_points_from_params(&req.params, image.timestamp);

    match classify_image_value(field, &r.value) {
        ImageValue::NotImage => (
            unconfirmed_item(
                req,
                &r,
                now,
                format!("field {field} is not an image payload"),
            ),
            true,
        ),
        ImageValue::Unreadable { reason } => (unconfirmed_item(req, &r, now, reason), true),
        ImageValue::Extracted {
            url,
            base64_len,
            mime: _,
        } => match bind_image_to_report(&anchor, field, &image, &siblings) {
            FrameBinding::Confirmed {
                report_id,
                image_field,
                proof,
            } => {
                let vision_path = url
                    .clone()
                    .unwrap_or_else(|| format!("inline_base64:len={}", base64_len.unwrap_or(0)));
                (
                    EvidenceItem {
                        key: req.key.clone(),
                        source_id: r.source_id,
                        result_type: req.result_type.clone(),
                        observed_at: r.observed_at.or(Some(image.timestamp)),
                        valid_window: r.valid_window,
                        fetched_at: now,
                        quality: EvidenceQuality::Fresh,
                        binding_proof: proof,
                        detail: Some(EvidenceDetail::Vision {
                            image_source_field: image_field,
                            vision_path: format!("{vision_path};report_id={report_id}"),
                        }),
                    },
                    false,
                )
            }
            FrameBinding::Unconfirmed { reason } => (unconfirmed_item(req, &r, now, reason), true),
        },
    }
}

fn sibling_points_from_params(
    params: &serde_json::Value,
    default_ts: i64,
) -> Vec<ReportMetricPoint> {
    let Some(arr) = params.get("report_siblings").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| {
            let name = v.get("name")?.as_str()?.to_string();
            let value = v.get("value")?.clone();
            let timestamp = v
                .get("timestamp")
                .and_then(|t| t.as_i64())
                .unwrap_or(default_ts);
            let metadata = v.get("metadata").cloned();
            Some(ReportMetricPoint {
                name,
                timestamp,
                value,
                metadata,
            })
        })
        .collect()
}

fn fresh_item(
    req: &InputRequirement,
    r: SourceReadResult,
    now: i64,
    image_field: Option<&str>,
) -> EvidenceItem {
    EvidenceItem {
        key: req.key.clone(),
        source_id: r.source_id,
        result_type: req.result_type.clone(),
        observed_at: r.observed_at,
        valid_window: r.valid_window,
        fetched_at: now,
        quality: EvidenceQuality::Fresh,
        binding_proof: r.binding_proof,
        detail: Some(detail_for(&req.result_type, r.value, image_field)),
    }
}

fn unconfirmed_item(
    req: &InputRequirement,
    r: &SourceReadResult,
    now: i64,
    reason: String,
) -> EvidenceItem {
    let quality = match req.on_missing {
        MissingPolicy::ReportStale => EvidenceQuality::Stale,
        MissingPolicy::Ignore => EvidenceQuality::Missing,
        _ => EvidenceQuality::Missing,
    };
    EvidenceItem {
        key: req.key.clone(),
        source_id: r.source_id.clone(),
        result_type: req.result_type.clone(),
        observed_at: r.observed_at,
        valid_window: r.valid_window,
        fetched_at: now,
        quality,
        binding_proof: format!("unconfirmed:{reason}"),
        detail: None,
    }
}

fn bump_ext_count(counts: &mut ExternalReadCounts, id: &str, op: &str) {
    let id_l = id.to_ascii_lowercase();
    let op_l = op.to_ascii_lowercase();
    if id_l.contains("weather")
        || op_l.contains("weather")
        || op_l == "current" && id_l.contains("weather")
    {
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

fn detail_for(
    modality: &InputModality,
    value: serde_json::Value,
    image_field: Option<&str>,
) -> EvidenceDetail {
    match modality {
        InputModality::Text => EvidenceDetail::Text {
            text: value.as_str().unwrap_or(&value.to_string()).to_string(),
        },
        InputModality::TimeSeries => {
            let points = value
                .as_array()
                .map(|a| a.len())
                .or_else(|| {
                    value
                        .get("points")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len())
                })
                .unwrap_or(0);
            EvidenceDetail::TimeSeries {
                points,
                value: Some(value),
            }
        }
        InputModality::Image => EvidenceDetail::Vision {
            image_source_field: image_field.unwrap_or("image").into(),
            vision_path: value
                .get("image_url")
                .or_else(|| value.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("inline")
                .to_string(),
        },
        _ => EvidenceDetail::Structured { value },
    }
}

fn missing_item(req: &InputRequirement, reason: String, now: i64) -> EvidenceItem {
    let quality = match req.on_missing {
        MissingPolicy::ReportStale => EvidenceQuality::Stale,
        MissingPolicy::Ignore => EvidenceQuality::Missing,
        _ => EvidenceQuality::Error {
            reason: reason.clone(),
        },
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
