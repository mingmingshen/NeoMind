use crate::types::{CliResponse, OutputFormat};

pub fn format_output(response: &CliResponse, format: OutputFormat) {
    let output = match format {
        OutputFormat::Json => serde_json::to_string_pretty(response).unwrap_or_default(),
        OutputFormat::Human => format_human(response),
    };
    print!("{}", output);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_output_json() {
        let response = CliResponse::success(json!({"id": "123", "name": "test"}), "Success");
        let output = format_output(&response, OutputFormat::Json);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["message"], "Success");
        assert_eq!(parsed["data"]["id"], "123");
        assert_eq!(parsed["data"]["name"], "test");
    }

    #[test]
    fn test_format_output_human_success() {
        let response = CliResponse::success(json!({"id": "123", "count": 42}), "Operation completed");
        let output = format_output(&response, OutputFormat::Human);

        assert!(output.contains("✅"));
        assert!(output.contains("Operation completed"));
        assert!(output.contains("id: 123"));
        assert!(output.contains("count: 42"));
    }

    #[test]
    fn test_format_output_human_error() {
        let response = CliResponse::error("Something failed", "ERR_001");
        let output = format_output(&response, OutputFormat::Human);

        assert!(output.contains("❌"));
        assert!(output.contains("Something failed"));
    }

    #[test]
    fn test_format_output_human_no_data() {
        let response = CliResponse::success(json!(null), "Done");
        let output = format_output(&response, OutputFormat::Human);

        assert!(output.contains("✅"));
        assert!(output.contains("Done"));
    }

    #[test]
    fn test_format_output_human_with_boolean() {
        let response = CliResponse::success(json!({"enabled": true, "active": false}), "Status");
        let output = format_output(&response, OutputFormat::Human);

        assert!(output.contains("enabled: true"));
        assert!(output.contains("active: false"));
    }

    #[test]
    fn test_format_output_json_error() {
        let response = CliResponse::error("Error message", "E001");
        let output = format_output(&response, OutputFormat::Json);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"], "Error message");
        assert_eq!(parsed["code"], "E001");
    }
}
