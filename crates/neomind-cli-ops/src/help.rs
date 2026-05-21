//! Embedded help manuals for each CLI domain.

/// Get the help manual for a domain. Returns None if domain is unknown.
pub fn get_help(domain: &str) -> Option<&'static str> {
    match domain {
        "device" => Some(include_str!("../../neomind-agent/src/skills/builtins/device-management.md")),
        "dashboard" => Some(include_str!("../../neomind-agent/src/skills/builtins/dashboard-management.md")),
        "rule" => Some(include_str!("../../neomind-agent/src/skills/builtins/rule-management.md")),
        "agent" => Some(include_str!("../../neomind-agent/src/skills/builtins/agent-management.md")),
        "transform" => Some(include_str!("../../neomind-agent/src/skills/builtins/transform-management.md")),
        "extension" => Some(include_str!("../../neomind-agent/src/skills/builtins/extension-management.md")),
        "message" => Some(include_str!("../../neomind-agent/src/skills/builtins/message-management.md")),
        "widget" => Some(include_str!("../../neomind-agent/src/skills/builtins/component-development.md")),
        "onboarding" => Some(include_str!("../../neomind-agent/src/skills/builtins/device-onboarding.md")),
        "system" => Some(include_str!("../../neomind-agent/src/skills/builtins/system-info.md")),
        _ => None,
    }
}

/// A domain entry for listing.
pub struct DomainInfo {
    pub name: &'static str,
    pub description: &'static str,
}

/// List all available help domains.
pub fn list_domains() -> Vec<DomainInfo> {
    vec![
        DomainInfo { name: "device", description: "Device management (list, create, control, metrics, types)" },
        DomainInfo { name: "dashboard", description: "Dashboard management (create, update, components, share)" },
        DomainInfo { name: "rule", description: "Rule engine (DSL syntax, create, enable, history)" },
        DomainInfo { name: "agent", description: "AI Agent management (create, schedule, control, executions)" },
        DomainInfo { name: "transform", description: "Data transform (JavaScript code, create, test)" },
        DomainInfo { name: "extension", description: "Extension management (install, status, marketplace)" },
        DomainInfo { name: "message", description: "Messages & notification channels (send, channels, webhook)" },
        DomainInfo { name: "widget", description: "Widget/component development (create, install, marketplace)" },
        DomainInfo { name: "onboarding", description: "Device onboarding guide (MQTT, webhook, connection methods)" },
        DomainInfo { name: "system", description: "System info & infrastructure (MQTT status, network, settings)" },
    ]
}
