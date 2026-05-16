use crate::types::{CliResponse, OutputFormat};

pub fn format_output(response: &CliResponse, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(response).unwrap_or_default(),
        OutputFormat::Human => format_human(response),
    }
}

fn format_human(resp: &CliResponse) -> String {
    if resp.success {
        let mut out = String::new();
        if let Some(msg) = &resp.message {
            out.push_str(&format!("✅ {}\n", msg));
        }
        if let Some(data) = &resp.data {
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    if let Some(s) = value.as_str() {
                        out.push_str(&format!("  {}: {}\n", key, s));
                    } else if value.is_number() || value.is_boolean() {
                        out.push_str(&format!("  {}: {}\n", key, value));
                    }
                }
            }
        }
        out
    } else {
        format!("❌ {}\n", resp.error.as_deref().unwrap_or("Unknown error"))
    }
}
