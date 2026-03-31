//! 统一的工具名称映射
//!
//! 此模块整合了分散在多个文件中的工具名称映射逻辑，确保:
//! - agent/mod.rs 中的 resolve_tool_name
//! - agent/streaming.rs 中的 resolve_tool_name
//!
//! 使用同一套映射规则

use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// 工具名称映射器
///
/// 负责将简化名称/别名映射到真实的工具名称
pub struct ToolNameMapper {
    /// 简化名称 -> 真实名称
    simplified_to_real: HashMap<String, String>,
    /// 别名 -> 真实名称
    alias_to_real: HashMap<String, String>,
}

impl ToolNameMapper {
    /// 创建新的映射器，包含所有内置映射
    pub fn new() -> Self {
        let mut mapper = Self {
            simplified_to_real: HashMap::new(),
            alias_to_real: HashMap::new(),
        };
        mapper.register_builtin_mappings();
        mapper
    }

    /// 注册内置的工具名称映射
    fn register_builtin_mappings(&mut self) {
        // ===== 聚合工具 (Aggregated Tools) =====
        // 新的 action-based 聚合工具，替代原来的独立工具
        // 这些工具名称直接映射到自身，避免被模糊匹配到旧工具
        self.register_simplified("device", "device");
        self.register_simplified("agent", "agent");
        self.register_simplified("agent_history", "agent_history");
        self.register_simplified("rule", "rule");
        self.register_simplified("alert", "alert");

        // ===== 设备工具别名 =====
        // 设备工具别名 - 指向聚合工具
        self.register_alias("设备列表", "device");
        self.register_alias("列出设备", "device");
        self.register_alias("查看设备", "device");
        self.register_alias("所有设备", "device");
        self.register_alias("发现设备", "device");

        // ===== 规则工具别名 =====
        self.register_alias("规则列表", "rule");
        self.register_alias("列出规则", "rule");
        self.register_alias("查看规则", "rule");
        self.register_alias("创建规则", "rule");
        self.register_alias("新建规则", "rule");
        self.register_alias("删除规则", "rule");

        // ===== Agent工具别名 =====
        self.register_alias("智能体列表", "agent");
        self.register_alias("列出智能体", "agent");
        self.register_alias("查看智能体", "agent");
        self.register_alias("所有智能体", "agent");
        self.register_alias("创建智能体", "agent");
        self.register_alias("新建智能体", "agent");

        // ===== 告警工具别名 =====
        self.register_alias("告警列表", "alert");
        self.register_alias("列出告警", "alert");
        self.register_alias("查看告警", "alert");
        self.register_alias("创建告警", "alert");

        // ===== 旧工具名称兼容映射 =====
        // 将旧工具名称映射到新的聚合工具
        self.register_alias("device_discover", "device");
        self.register_alias("device_query", "device");
        self.register_alias("device_control", "device");
        self.register_alias("device_analyze", "device");
        self.register_alias("list_devices", "device");
        self.register_alias("get_device_data", "device");
        self.register_alias("query_data", "device");
        self.register_alias("control_device", "device");

        self.register_alias("list_rules", "rule");
        self.register_alias("create_rule", "rule");
        self.register_alias("delete_rule", "rule");
        self.register_alias("get_rule", "rule");

        self.register_alias("list_agents", "agent");
        self.register_alias("get_agent", "agent");
        self.register_alias("create_agent", "agent");
        self.register_alias("execute_agent", "agent");
        self.register_alias("control_agent", "agent");

        self.register_alias("list_alerts", "alert");
        self.register_alias("create_alert", "alert");
        self.register_alias("acknowledge_alert", "alert");

        // ===== 旧工具名称兼容映射 =====
        // 将旧工具名称映射到新的聚合工具
        self.register_alias("list_workflows", "rule");  // 工作流暂用rule
        self.register_alias("create_workflow", "rule");
        self.register_alias("trigger_workflow", "rule");

        self.register_alias("list_scenarios", "rule");  // 场景暂用rule
        self.register_alias("create_scenario", "rule");
        self.register_alias("execute_scenario", "rule");
    }

    /// 注册简化名称映射
    fn register_simplified(&mut self, simplified: &str, real: &str) {
        self.simplified_to_real
            .insert(simplified.to_string(), real.to_string());
    }

    /// 注册别名映射
    fn register_alias(&mut self, alias: &str, real: &str) {
        self.alias_to_real
            .insert(alias.to_string(), real.to_string());
    }

    /// 解析工具名称
    ///
    /// 将简化名称或别名解析为真实的工具名称
    pub fn resolve(&self, input: &str) -> String {
        // 1. 简化名称映射 (如 device.discover -> device.discover 自身)
        //    优先检查这个，因为它是 LLM 最常用的格式
        if let Some(real) = self.simplified_to_real.get(input) {
            return real.clone();
        }

        // 2. 别名映射 (如 "设备列表" -> device.discover, "list_devices" -> device.discover)
        if let Some(real) = self.alias_to_real.get(input) {
            return real.clone();
        }

        // 3. 模糊匹配 - 检查是否包含已知别名的一部分
        if let Some(real) = self.fuzzy_match(input) {
            return real;
        }

        // 4. 默认返回输入，假设它是真实名称
        input.to_string()
    }

    /// 模糊匹配
    ///
    /// 对于部分匹配的别名，尝试找到最相似的工具
    fn fuzzy_match(&self, input: &str) -> Option<String> {
        // 精确子串匹配
        for (alias, real) in &self.alias_to_real {
            if alias.contains(input) || input.contains(alias) {
                return Some(real.clone());
            }
        }

        // 对于简化名称也做同样的检查
        for (simplified, real) in &self.simplified_to_real {
            if simplified.contains(input) || input.contains(simplified) {
                return Some(real.clone());
            }
        }

        None
    }

    /// 获取工具的所有别名
    pub fn get_aliases(&self, real_name: &str) -> Vec<String> {
        let mut aliases = Vec::new();

        // 收集所有指向该真实名称的别名
        for (alias, real) in &self.alias_to_real {
            if real == real_name {
                aliases.push(alias.clone());
            }
        }

        // 收集所有指向该真实名称的简化名称
        for (simplified, real) in &self.simplified_to_real {
            if real == real_name {
                aliases.push(simplified.clone());
            }
        }

        aliases
    }

    /// 注册自定义映射
    ///
    /// 允许运行时添加新的工具名称映射
    pub fn register_custom(&mut self, alias: String, real_name: String) {
        self.alias_to_real.insert(alias, real_name);
    }

    /// 获取所有已注册的工具名称
    pub fn all_known_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // 真实名称（从映射值中推断）
        for real in self.simplified_to_real.values() {
            if !names.contains(real) {
                names.push(real.clone());
            }
        }
        for real in self.alias_to_real.values() {
            if !names.contains(real) {
                names.push(real.clone());
            }
        }

        names.sort();
        names
    }

    /// 检查是否包含某个别名
    pub fn has_alias(&self, alias: &str) -> bool {
        self.alias_to_real.contains_key(alias)
    }

    /// 检查是否包含某个简化名称
    pub fn has_simplified(&self, simplified: &str) -> bool {
        self.simplified_to_real.contains_key(simplified)
    }
}

impl Default for ToolNameMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局工具名称映射器
static GLOBAL_MAPPER: OnceLock<ToolNameMapper> = OnceLock::new();

/// 获取全局工具名称映射器
pub fn get_mapper() -> &'static ToolNameMapper {
    GLOBAL_MAPPER.get_or_init(ToolNameMapper::new)
}

/// 解析工具名称（便捷函数）
///
/// 使用全局映射器将简化名称或别名解析为真实工具名称
pub fn resolve_tool_name(input: &str) -> String {
    get_mapper().resolve(input)
}

/// 映射工具参数
///
/// 将简化参数名映射到真实参数名
/// 支持新旧工具名称的向后兼容
pub fn map_tool_parameters(tool_name: &str, arguments: &Value) -> Value {
    let real_tool_name = resolve_tool_name(tool_name);

    if let Some(obj) = arguments.as_object() {
        let mut mapped = serde_json::Map::new();

        for (key, value) in obj {
            let actual_key = match (real_tool_name.as_str(), key.as_str()) {
                // ===== device tool (aggregated) =====
                // Backward compatibility for old parameter names
                ("device", "device") => "device_id",
                ("device", "action") => {
                    // If action looks like a control command (on/off/set), map to command
                    if let Some(action_val) = value.as_str() {
                        if ["on", "off", "set", "toggle", "open", "close"].contains(&action_val) {
                            mapped.insert("command".to_string(), value.clone());
                            continue;
                        }
                    }
                    // Otherwise keep as action
                    "action"
                }
                ("device", "value") => "params",
                ("device", "hours") => {
                    // 将小时转换为时间戳
                    if let Some(hours) = value.as_i64() {
                        let end_time = chrono::Utc::now().timestamp();
                        let start_time = end_time - (hours * 3600);
                        mapped.insert("end_time".to_string(), serde_json::json!(end_time));
                        mapped.insert("start_time".to_string(), serde_json::json!(start_time));
                        continue;
                    }
                    "start_time"
                }
                // Old filter params -> new flat params
                ("device", "type") => "device_type",
                ("device", "status") => "status", // keep as-is for list action
                ("device", other) => other,

                // ===== rule tool (aggregated) =====
                ("rule", "rule") => "rule_id",
                ("rule", "dsl") => "dsl",
                ("rule", other) => other,

                // ===== agent tool (aggregated) =====
                ("agent", "agent") => "agent_id",
                ("agent", other) => other,

                // ===== alert tool (aggregated) =====
                ("alert", "alert") => "alert_id",
                ("alert", other) => other,

                // ===== Legacy tool names (now map to aggregated) =====
                // These are kept for backward compatibility if old names are used directly

                // query_data -> device (with action=query)
                ("query_data", "device") => "device_id",
                ("query_data", "hours") => {
                    if let Some(hours) = value.as_i64() {
                        let end_time = chrono::Utc::now().timestamp();
                        let start_time = end_time - (hours * 3600);
                        mapped.insert("end_time".to_string(), serde_json::json!(end_time));
                        mapped.insert("start_time".to_string(), serde_json::json!(start_time));
                        continue;
                    }
                    "start_time"
                }
                ("query_data", other) => other,

                // control_device -> device (with action=control)
                ("control_device", "device") => "device_id",
                ("control_device", "action") => "command",
                ("control_device", "value") => "params",
                ("control_device", other) => other,

                // device_control -> device (with action=control)
                ("device_control", "device") => "device_id",
                ("device_control", "action") => "command",
                ("device_control", "value") => "params",
                ("device_control", other) => other,

                // create_rule -> rule (with action=create)
                ("create_rule", other) => other,

                // disable_rule / enable_rule -> rule
                ("disable_rule", "rule") | ("enable_rule", "rule") => "rule_id",
                ("disable_rule", other) | ("enable_rule", other) => other,

                // list_devices -> device (with action=list)
                ("list_devices", "type") => "device_type",
                ("list_devices", other) => other,

                // device_discover -> device (with action=list)
                ("device_discover", "type") => "device_type",
                ("device_discover", other) => other,

                // Default: keep original key
                _ => key.as_str(),
            };

            mapped.insert(actual_key.to_string(), value.clone());
        }

        Value::Object(mapped)
    } else {
        arguments.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_tool_mapping() {
        let mapper = ToolNameMapper::new();
        // Aggregated tool names map to themselves
        assert_eq!(mapper.resolve("device"), "device");
        assert_eq!(mapper.resolve("agent"), "agent");
        assert_eq!(mapper.resolve("agent_history"), "agent_history");
        assert_eq!(mapper.resolve("rule"), "rule");
        assert_eq!(mapper.resolve("alert"), "alert");
    }

    #[test]
    fn test_legacy_tool_compatibility() {
        let mapper = ToolNameMapper::new();
        // Old tool names map to new aggregated tools
        assert_eq!(mapper.resolve("device_discover"), "device");
        assert_eq!(mapper.resolve("device_query"), "device");
        assert_eq!(mapper.resolve("device_control"), "device");
        assert_eq!(mapper.resolve("list_devices"), "device");
        assert_eq!(mapper.resolve("get_device_data"), "device");
        assert_eq!(mapper.resolve("query_data"), "device");

        assert_eq!(mapper.resolve("list_rules"), "rule");
        assert_eq!(mapper.resolve("create_rule"), "rule");
        assert_eq!(mapper.resolve("delete_rule"), "rule");

        assert_eq!(mapper.resolve("list_agents"), "agent");
        assert_eq!(mapper.resolve("get_agent"), "agent");
        assert_eq!(mapper.resolve("create_agent"), "agent");

        assert_eq!(mapper.resolve("list_alerts"), "alert");
        assert_eq!(mapper.resolve("create_alert"), "alert");
    }

    #[test]
    fn test_chinese_aliases() {
        let mapper = ToolNameMapper::new();
        // Chinese aliases map to aggregated tools
        assert_eq!(mapper.resolve("设备列表"), "device");
        assert_eq!(mapper.resolve("列出设备"), "device");
        assert_eq!(mapper.resolve("查看设备"), "device");
        assert_eq!(mapper.resolve("所有设备"), "device");

        assert_eq!(mapper.resolve("规则列表"), "rule");
        assert_eq!(mapper.resolve("列出规则"), "rule");

        assert_eq!(mapper.resolve("智能体列表"), "agent");
        assert_eq!(mapper.resolve("列出智能体"), "agent");

        assert_eq!(mapper.resolve("告警列表"), "alert");
    }

    #[test]
    fn test_real_name_passthrough() {
        let mapper = ToolNameMapper::new();
        // Unknown names should pass through
        assert_eq!(mapper.resolve("unknown_tool"), "unknown_tool");
        assert_eq!(mapper.resolve("custom_function"), "custom_function");
    }

    #[test]
    fn test_get_aliases() {
        let mapper = ToolNameMapper::new();
        let aliases = mapper.get_aliases("device");
        assert!(aliases.contains(&"设备列表".to_string()));
        assert!(aliases.contains(&"list_devices".to_string()));
    }

    #[test]
    fn test_parameter_mapping_device() {
        let args = serde_json::json!({
            "device": "lamp_1",
            "action": "control",
            "command": "on",
            "value": "100"
        });

        // New aggregated device tool mapping
        let mapped = map_tool_parameters("device", &args);
        assert_eq!(mapped.get("device_id").unwrap(), "lamp_1");
        assert_eq!(mapped.get("command").unwrap(), "on");
        assert_eq!(mapped.get("params").unwrap(), "100");

        // Legacy control_device still works (maps to device tool)
        let mapped = map_tool_parameters("control_device", &args);
        assert_eq!(mapped.get("device_id").unwrap(), "lamp_1");
        assert_eq!(mapped.get("command").unwrap(), "on");
    }

    #[test]
    fn test_parameter_mapping_data() {
        let args = serde_json::json!({
            "device": "sensor_1",
            "hours": 24
        });

        // New aggregated device tool with query action
        let mapped = map_tool_parameters("device", &args);
        assert_eq!(mapped.get("device_id").unwrap(), "sensor_1");
        // hours should be converted to start_time and end_time
        assert!(mapped.get("start_time").is_some());
        assert!(mapped.get("end_time").is_some());

        // Legacy query_data still works
        let mapped = map_tool_parameters("query_data", &args);
        assert_eq!(mapped.get("device_id").unwrap(), "sensor_1");
        assert!(mapped.get("start_time").is_some());
        assert!(mapped.get("end_time").is_some());
    }

    #[test]
    fn test_global_mapper() {
        // 测试全局映射器 - 旧名称映射到新聚合工具
        let resolved = resolve_tool_name("device_discover");
        assert_eq!(resolved, "device");
    }

    #[test]
    fn test_custom_registration() {
        let mut mapper = ToolNameMapper::new();
        mapper.register_custom("custom_alias".to_string(), "device".to_string());
        assert_eq!(mapper.resolve("custom_alias"), "device");
    }

    #[test]
    fn test_all_known_names() {
        let mapper = ToolNameMapper::new();
        let names = mapper.all_known_names();
        // Should contain aggregated tool names
        assert!(names.contains(&"device".to_string()));
        assert!(names.contains(&"rule".to_string()));
        assert!(names.contains(&"agent".to_string()));
    }

    #[test]
    fn test_parameter_mapping_device_discover_filter() {
        // Test that type/status are mapped to flat parameters for new aggregated tool
        let args = serde_json::json!({
            "type": "sensor",
            "status": "online"
        });

        let mapped = map_tool_parameters("device_discover", &args);

        // Should have flat device_type parameter (new aggregated format)
        assert_eq!(mapped.get("device_type").unwrap(), "sensor");
        assert_eq!(mapped.get("status").unwrap(), "online");
    }

    #[test]
    fn test_parameter_mapping_device_discover_with_group_by() {
        // Test that group_by is preserved
        let args = serde_json::json!({
            "type": "sensor",
            "group_by": "type"
        });

        let mapped = map_tool_parameters("device_discover", &args);

        // group_by should be at top level
        assert_eq!(mapped.get("group_by").unwrap(), "type");
        // type should be mapped to device_type
        assert_eq!(mapped.get("device_type").unwrap(), "sensor");
    }

    #[test]
    fn test_parameter_mapping_list_devices_legacy() {
        // Test that legacy list_devices maps to flat parameters
        let args = serde_json::json!({
            "type": "sensor",
            "status": "online"
        });

        let mapped = map_tool_parameters("list_devices", &args);

        // Should have flat device_type parameter
        assert_eq!(mapped.get("device_type").unwrap(), "sensor");
        assert_eq!(mapped.get("status").unwrap(), "online");
    }
}
