//! Aggregate scores.jsonl → grade-card.md.
//! Spec §11 format. Spec §13 step 6: quarantine malformed lines.
use crate::record::ScoreLine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregate {
    pub total_cases: usize,
    pub malformed: usize,
    pub by_dimension: HashMap<String, Vec<f64>>,
    pub by_lang: HashMap<String, Vec<f64>>,
    pub overall_per_case: Vec<f64>,
    pub agent_errors: usize,
    pub suspected_fallback: usize,
}

impl Aggregate {
    pub fn dimension_averages(&self) -> HashMap<String, f64> {
        self.by_dimension
            .iter()
            .map(|(k, vs)| (k.clone(), vs.iter().sum::<f64>() / vs.len() as f64))
            .collect()
    }
    pub fn malformed_rate(&self) -> f64 {
        let denom = self.total_cases + self.malformed;
        if denom == 0 {
            0.0
        } else {
            self.malformed as f64 / denom as f64
        }
    }
    pub fn overall(&self) -> f64 {
        if self.overall_per_case.is_empty() {
            0.0
        } else {
            self.overall_per_case.iter().sum::<f64>() / self.overall_per_case.len() as f64
        }
    }
}

pub fn aggregate(jsonl: &str) -> anyhow::Result<Aggregate> {
    let mut agg = Aggregate {
        total_cases: 0,
        malformed: 0,
        agent_errors: 0,
        suspected_fallback: 0,
        by_dimension: HashMap::new(),
        by_lang: HashMap::new(),
        overall_per_case: Vec::new(),
    };
    for line in jsonl.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ScoreLine>(line) {
            Ok(s) => {
                agg.total_cases += 1;
                if s.suspected_fallback {
                    agg.suspected_fallback += 1;
                }
                if s.status.as_deref() == Some("agent_error")
                    || s.status.as_deref() == Some("runtime_error")
                    || s.status.as_deref() == Some("seed_failure")
                    || s.status.as_deref() == Some("llm_config_error")
                    || s.status.as_deref() == Some("agent_timeout")
                {
                    agg.agent_errors += 1;
                }
                if let Some(obj) = s.scores.as_object() {
                    let present: Vec<(&String, &serde_json::Number)> = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_number().map(|n| (k, n)))
                        .collect();
                    let weight = |dim: &str| -> f64 {
                        match dim {
                            "tool_accuracy" => 25.0,
                            "task_completion" => 25.0,
                            "response_quality" => 20.0,
                            "context_retention" => 15.0,
                            "error_recovery" => 15.0,
                            "language_adherence" => 5.0,
                            _ => 0.0,
                        }
                    };
                    let total_w: f64 = present.iter().map(|(k, _)| weight(k)).sum();
                    let mut case_overall = 0.0;
                    for (k, n) in &present {
                        let score = n.as_f64().unwrap_or(0.0);
                        agg.by_dimension.entry((*k).clone()).or_default().push(score);
                        if total_w > 0.0 {
                            case_overall += score * weight(k) / total_w;
                        }
                    }
                    // case_overall is 0-10; convert to 0-100.
                    agg.overall_per_case.push(case_overall * 10.0);
                    agg.by_lang
                        .entry(s.lang.clone())
                        .or_default()
                        .push(case_overall * 10.0);
                }
            }
            Err(_) => agg.malformed += 1,
        }
    }
    Ok(agg)
}

pub fn write_grade_card(agg: &Aggregate, out_path: &Path) -> anyhow::Result<()> {
    let mut md = String::new();
    md.push_str("# NeoMind Chat Eval Report\n\n");
    let grade = grade_letter(agg.overall());
    md.push_str(&format!(
        "## Overall Grade: **{} ({:.1})**\n\n",
        grade,
        agg.overall()
    ));
    if agg.malformed_rate() > 0.05 {
        md.push_str(&format!(
            "⚠️ Malformed score lines: {:.1}% — results may be unreliable.\n\n",
            agg.malformed_rate() * 100.0
        ));
    }
    if agg.agent_errors > 0 {
        md.push_str(&format!(
            "⚠️ Agent failures: {} case(s) excluded from averages.\n\n",
            agg.agent_errors
        ));
    }
    md.push_str("| Dimension | Avg (0-10) |\n|---|---|\n");
    for (dim, avg) in agg.dimension_averages() {
        md.push_str(&format!("| {} | {:.2} |\n", dim, avg));
    }
    md.push_str("\n## By Language\n\n| Lang | Cases | Avg (0-100) |\n|---|---|---|\n");
    for (lang, vs) in &agg.by_lang {
        let avg = vs.iter().sum::<f64>() / vs.len() as f64;
        md.push_str(&format!("| {} | {} | {:.1} |\n", lang, vs.len(), avg));
    }
    md.push_str(&format!(
        "\nSuspected fallback cases: {}\n",
        agg.suspected_fallback
    ));
    std::fs::write(out_path, md)?;
    Ok(())
}

pub fn grade_letter(score: f64) -> &'static str {
    if score >= 85.0 {
        "A"
    } else if score >= 70.0 {
        "B"
    } else if score >= 55.0 {
        "C"
    } else if score >= 40.0 {
        "D"
    } else {
        "F"
    }
}
