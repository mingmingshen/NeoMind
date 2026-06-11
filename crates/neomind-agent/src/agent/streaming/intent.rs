// ---------------------------------------------------------------------------
// "List-only dead end" detection helpers
// ---------------------------------------------------------------------------

/// Action verbs (in Chinese and English) that indicate the user wants a mutation,
/// not just a query. When the original user message contains these but all executed
/// tool calls were read-only (list/get/latest/history), we detect a "list-only dead end"
/// and inject a forced continuation prompt.
const ACTION_VERBS: &[&str] = &[
    // Chinese
    "创建",
    "新建",
    "删除",
    "控制",
    "启用",
    "禁用",
    "启动",
    "停止",
    "更新",
    "修改",
    "开启",
    "关闭",
    "打开",
    "发送",
    "写入",
    "分享",
    "安装",
    "卸载",
    "移除",
    "批量启用",
    "批量删除",
    "批量创建",
    "全部启动",
    "添加",
    "替换",
    "绑定",
    "检查",
    "巡检",
    // English
    "create",
    "delete",
    "control",
    "enable",
    "disable",
    "start",
    "stop",
    "update",
    "turn on",
    "turn off",
    "switch",
    "send",
    "write",
    "share",
    "install",
    "uninstall",
    "remove",
    "add",
    "replace",
    "bind",
];

/// Check if the user message requests a mutation/action (not just a query).
pub(crate) fn user_message_requires_action(msg: &str) -> bool {
    let msg_lower = msg.to_lowercase();
    ACTION_VERBS.iter().any(|verb| msg_lower.contains(verb))
}

/// Check if ALL executed tool calls so far were read-only (list/get/query).
/// Takes the actual shell command strings (not tool names) for accurate detection.
/// Returns true if no mutation command was found in any tool call.
pub(crate) fn all_tools_were_read_only(executed_commands: &[&str], _all_results: &[(String, String)]) -> bool {
    // Mutation command patterns — if ANY of these appear, it's NOT read-only
    const MUTATION_COMMANDS: &[&str] = &[
        " create",
        " delete",
        " update",
        " control",
        " enable",
        " disable",
        " write-metric",
        " send-message",
        " share",
        " install ",
        " uninstall ",
        " control ",
        " send ",
        " channel-create",
        " channel-update",
        " channel-delete",
    ];

    // If no commands were executed, we can't determine — assume not read-only
    if executed_commands.is_empty() {
        return false;
    }

    for cmd in executed_commands {
        let cmd_lower = cmd.to_lowercase();
        // Check if this command is a mutation
        let is_mutation = MUTATION_COMMANDS.iter().any(|m| cmd_lower.contains(m));
        if is_mutation {
            return false;
        }
    }

    // All commands were read-only (list/get/latest/history/etc.)
    true
}

/// Extract the domain and expected action from the user message for the forced prompt.
pub(crate) fn extract_action_hint(msg: &str) -> String {
    let msg_lower = msg.to_lowercase();

    // Domain detection
    let domain = if msg_lower.contains("规则") || msg_lower.contains("rule") {
        "rule"
    } else if msg_lower.contains("agent")
        || msg_lower.contains("代理")
        || msg_lower.contains("智能体")
    {
        "agent"
    } else if msg_lower.contains("设备")
        || msg_lower.contains("device")
        || msg_lower.contains("sensor")
    {
        "device"
    } else if msg_lower.contains("仪表盘")
        || msg_lower.contains("仪表板")
        || msg_lower.contains("dashboard")
        || msg_lower.contains("面板")
    {
        "dashboard"
    } else if msg_lower.contains("转换") || msg_lower.contains("transform") {
        "transform"
    } else if msg_lower.contains("组件")
        || msg_lower.contains("widget")
        || msg_lower.contains("小部件")
    {
        "widget"
    } else if msg_lower.contains("扩展")
        || msg_lower.contains("extension")
        || msg_lower.contains("插件")
    {
        "extension"
    } else if msg_lower.contains("消息")
        || msg_lower.contains("message")
        || msg_lower.contains("通知")
        || msg_lower.contains("通道")
        || msg_lower.contains("channel")
    {
        "message"
    } else {
        ""
    };

    // Action detection
    let action =
        if msg_lower.contains("创建") || msg_lower.contains("create") || msg_lower.contains("新建")
        {
            "create"
        } else if msg_lower.contains("删除")
            || msg_lower.contains("delete")
            || msg_lower.contains("移除")
        {
            "delete"
        } else if msg_lower.contains("控制")
            || msg_lower.contains("control")
            || msg_lower.contains("打开")
            || msg_lower.contains("关闭")
            || msg_lower.contains("开启")
        {
            "control"
        } else if msg_lower.contains("启用")
            || msg_lower.contains("enable")
            || msg_lower.contains("启动")
            || msg_lower.contains("start")
        {
            "enable/start"
        } else if msg_lower.contains("禁用")
            || msg_lower.contains("disable")
            || msg_lower.contains("停止")
            || msg_lower.contains("stop")
        {
            "disable/stop"
        } else if msg_lower.contains("更新")
            || msg_lower.contains("update")
            || msg_lower.contains("修改")
            || msg_lower.contains("替换")
        {
            "update"
        } else if msg_lower.contains("写入")
            || msg_lower.contains("write")
            || msg_lower.contains("发送")
            || msg_lower.contains("send")
        {
            "write/send"
        } else if msg_lower.contains("添加") || msg_lower.contains("add") {
            "add"
        } else if msg_lower.contains("安装") || msg_lower.contains("install") {
            "install"
        } else if msg_lower.contains("卸载") || msg_lower.contains("uninstall") {
            "uninstall"
        } else if msg_lower.contains("分享") || msg_lower.contains("share") {
            "share"
        } else {
            ""
        };

    if domain.is_empty() && action.is_empty() {
        String::new()
    } else if domain.is_empty() {
        format!("the {} action", action)
    } else if action.is_empty() {
        format!("neomind {}", domain)
    } else {
        format!("neomind {} {}", domain, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_message_requires_action_chinese() {
        assert!(user_message_requires_action("创建新设备"));
        assert!(user_message_requires_action("删除旧配置"));
        assert!(user_message_requires_action("启用监控"));
        assert!(user_message_requires_action("发送消息"));
        assert!(!user_message_requires_action("列出所有设备"));
        assert!(!user_message_requires_action("查看状态"));
    }

    #[test]
    fn test_user_message_requires_action_english() {
        assert!(user_message_requires_action("create device"));
        assert!(user_message_requires_action("delete old config"));
        assert!(user_message_requires_action("STOP the service"));
        assert!(!user_message_requires_action("list devices"));
        assert!(!user_message_requires_action("get status"));
    }

    #[test]
    fn test_all_tools_were_read_only() {
        assert!(all_tools_were_read_only(&["neomind device list", "neomind rule list"], &[]));
        assert!(!all_tools_were_read_only(&["neomind device create sensor1"], &[]));
        assert!(!all_tools_were_read_only(&["neomind device list", "neomind device delete sensor1"], &[]));
        assert!(!all_tools_were_read_only(&[], &[])); // empty = not read-only
    }

    #[test]
    fn test_extract_action_hint() {
        assert_eq!(extract_action_hint("创建新规则"), "neomind rule create");
        assert_eq!(extract_action_hint("删除设备"), "neomind device delete");
        assert_eq!(extract_action_hint("创建仪表盘"), "neomind dashboard create");
        assert_eq!(extract_action_hint("查看状态"), "");
    }
}
