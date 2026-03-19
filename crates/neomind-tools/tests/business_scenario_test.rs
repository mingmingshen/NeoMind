//! Business scenario integration tests for core tools.
//!
//! This module tests real-world business scenarios using the new core tools:
//! 1. Device discovery and exploration
//! 2. Device data querying and analysis
//! 3. Device control operations
//! 4. Multi-step workflows combining multiple tools

use neomind_tools::core_tools::*;
use neomind_tools::Tool;
use serde_json::json;
use std::sync::Arc;

/// Scenario 1: User asks "What devices do I have?"
///
/// Flow: device.discover() → List all devices with summary
#[tokio::test]
async fn scenario_1_discover_all_devices() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let tool = DeviceDiscoverTool::new(registry);

    let result = tool
        .execute(json!({}))
        .await
        .expect("discover should succeed");

    assert!(result.success);
    let data = &result.data;

    // Verify summary
    assert_eq!(data["summary"]["total"], 7);
    assert_eq!(data["summary"]["online"], 6);
    assert_eq!(data["summary"]["offline"], 1);

    println!("=== 场景1: 发现所有设备 ===");
    println!("总计: {} 台设备", data["summary"]["total"]);
    println!("在线: {} 台", data["summary"]["online"]);
    println!("离线: {} 台", data["summary"]["offline"]);

    // Print device types
    if let Some(by_type) = data["summary"]["by_type"].as_object() {
        println!("\n按类型分类:");
        for (type_name, count) in by_type {
            println!("  - {}: {} 台", type_name, count);
        }
    }

    // Print locations
    if let Some(by_location) = data["summary"]["by_location"].as_object() {
        println!("\n按位置分类:");
        for (location, count) in by_location {
            println!("  - {}: {} 台", location, count);
        }
    }
}

/// Scenario 2: User asks "What sensors are available?"
///
/// Flow: device.discover(filter: {type: "sensor"}) → List all sensors
#[tokio::test]
async fn scenario_2_discover_by_type() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let tool = DeviceDiscoverTool::new(registry);

    let result = tool
        .execute(json!({
            "filter": {"type": "sensor"},
            "group_by": "none",
            "include_data_preview": true
        }))
        .await
        .expect("discover should succeed");

    assert!(result.success);
    let data = &result.data;

    println!("\n=== 场景2: 查询所有传感器 ===");

    if let Some(groups) = data["groups"].as_array() {
        for group in groups {
            println!("\n{} ({} 台):", group["name"], group["count"]);
            if let Some(devices) = group["devices"].as_array() {
                for device in devices {
                    println!("  - [{}] {}", device["id"], device["name"]);
                    if let Some(latest_data) = device["latest_data"].as_object() {
                        println!("    当前数据: {:?}", latest_data);
                    }
                }
            }
        }
    }
}

/// Scenario 3: User asks "What's the temperature in the living room?"
///
/// Flow: device.query(device_id: "sensor_temp_living", metrics: ["temperature"])
#[tokio::test]
async fn scenario_3_query_temperature() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let tool = DeviceQueryTool::new(registry);

    let result = tool
        .execute(json!({
            "device_id": "sensor_temp_living",
            "metrics": ["temperature"]
        }))
        .await
        .expect("query should succeed");

    assert!(result.success);
    let data = &result.data;

    println!("\n=== 场景3: 查询客厅温度 ===");
    println!("设备: {} ({})", data["device_name"], data["device_id"]);

    if let Some(metrics) = data["metrics"].as_array() {
        for metric in metrics {
            println!("指标: {} ({})", metric["display_name"], metric["unit"]);
            if let Some(current) = metric["current"].as_f64() {
                println!("当前值: {:.1}", current);
            }
            if let Some(stats) = metric["stats"].as_object() {
                println!(
                    "统计: 平均={:.1}°C, 最低={:.1}°C, 最高={:.1}°C",
                    stats["avg"], stats["min"], stats["max"]
                );
            }
            if let Some(hint) = metric["analysis_hint"].as_str() {
                println!("分析: {}", hint);
            }
        }
    }
}

/// Scenario 4: User asks "Turn on the living room light"
///
/// Flow: device.control(device_id: "light_living_main", command: "turn_on")
#[tokio::test]
async fn scenario_4_control_single_device() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let tool = DeviceControlTool::new(registry);

    let result = tool
        .execute(json!({
            "device_id": "light_living_main",
            "command": "turn_on"
        }))
        .await
        .expect("control should succeed");

    assert!(result.success);
    let data = &result.data;

    println!("\n=== 场景4: 控制客厅灯 ===");
    println!("命令: {}", data["command"]);
    println!("成功: {} 个", data["successful"]);
    println!("确认信息: {}", data["confirmation"]);

    assert_eq!(data["successful"], 1);
    assert!(data["confirmation"].as_str().unwrap().contains("打开"));
}

/// Scenario 5: User asks "Turn on all lights"
///
/// Flow: device.control(device_id: "light", command: "turn_on")
/// Note: Uses fuzzy matching to find all devices with "light" in name/id
#[tokio::test]
async fn scenario_5_batch_control_lights() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let tool = DeviceControlTool::new(registry);

    let result = tool
        .execute(json!({
            "device_id": "light",  // Fuzzy match all devices with "light"
            "command": "turn_on"
        }))
        .await
        .expect("control should succeed");

    assert!(result.success);
    let data = &result.data;

    println!("\n=== 场景5: 批量控制所有灯 ===");
    println!("目标设备: {} 个", data["total_targets"]);
    println!("成功: {} 个", data["successful"]);

    if let Some(results) = data["results"].as_array() {
        println!("\n执行结果:");
        for result in results {
            let status = if result["success"].as_bool().unwrap() {
                "✓"
            } else {
                "✗"
            };
            println!(
                "  {} [{}] {}",
                status, result["device_id"], result["device_name"]
            );
        }
    }

    println!("\n确认信息: {}", data["confirmation"]);
}

/// Scenario 6: Multi-step workflow - User says "Temperature is high, turn on AC and create an alert rule"
///
/// Flow:
/// 1. device.discover() - Find temperature sensors and AC
/// 2. device.query() - Get current temperature
/// 3. device.control() - Turn on AC if temperature is high
/// 4. (In real system) rule.from_context() - Create alert rule
#[tokio::test]
async fn scenario_6_multi_step_temperature_workflow() {
    let registry = Arc::new(MockDeviceRegistry::new());

    println!("\n=== 场景6: 多步骤工作流 - 温度高处理 ===");

    // Step 1: Discover temperature sensors
    let discover_tool = DeviceDiscoverTool::new(registry.clone());
    let discover_result = discover_tool
        .execute(json!({
            "filter": {"tags": ["sensor", "temperature"]},
            "include_data_preview": true
        }))
        .await
        .expect("discover should succeed");

    println!("\n步骤 1: 发现温度传感器");
    let groups = discover_result.data["groups"].as_array().unwrap();
    let mut sensor_id = String::new();
    let mut temp_value = 0.0;

    for group in groups {
        if let Some(devices) = group["devices"].as_array() {
            for device in devices {
                println!("  - [{}] {}", device["id"], device["name"]);
                if let Some(data) = device["latest_data"].as_object() {
                    if let Some(temp) = data.get("temperature").and_then(|v| v.as_f64()) {
                        println!("    温度: {:.1}°C", temp);
                        if temp > 26.0 && sensor_id.is_empty() {
                            sensor_id = device["id"].as_str().unwrap().to_string();
                            temp_value = temp;
                        }
                    }
                }
            }
        }
    }

    // Step 2: Query detailed temperature data
    let query_tool = DeviceQueryTool::new(registry.clone());
    println!("\n步骤 2: 查询温度详情");

    let query_result = query_tool
        .execute(json!({
            "device_id": &sensor_id,
            "metrics": ["temperature", "humidity"]
        }))
        .await
        .expect("query should succeed");

    if let Some(metrics) = query_result.data["metrics"].as_array() {
        for metric in metrics {
            let current = metric["current"].as_f64().unwrap_or(0.0);
            println!(
                "  {}: 当前值 {:.1}{}",
                metric["display_name"], current, metric["unit"]
            );
        }
    }

    // Step 3: Control AC (find and turn on)
    let control_tool = DeviceControlTool::new(registry.clone());
    println!("\n步骤 3: 查找并控制空调");

    // Find AC devices
    let all_devices = registry.get_all().await;
    let ac_devices: Vec<_> = all_devices
        .iter()
        .filter(|d| d.device_type == "AirConditioner")
        .collect();

    if !ac_devices.is_empty() {
        let ac = &ac_devices[0];
        println!("  找到空调: [{}] {}", ac.id, ac.name);

        let control_result = control_tool
            .execute(json!({
                "device_id": &ac.id,
                "command": "turn_on"
            }))
            .await
            .expect("control should succeed");

        if control_result.success {
            println!("  ✓ 已开启空调");
        }
    }

    println!("\n工作流完成: 温度 {:.1}°C → 已开启空调", temp_value);
}

/// Scenario 7: User asks "Show me sensor data from all rooms"
///
/// Flow:
/// 1. device.discover() - Find all sensors by location
/// 2. device.query() - Query each sensor's data
#[tokio::test]
async fn scenario_7_all_rooms_sensor_data() {
    let registry = Arc::new(MockDeviceRegistry::new());

    println!("\n=== 场景7: 查询所有房间的传感器数据 ===");

    // First, discover devices grouped by location
    let discover_tool = DeviceDiscoverTool::new(registry.clone());
    let discover_result = discover_tool
        .execute(json!({
            "filter": {"tags": ["sensor"]},
            "group_by": "location",
            "include_data_preview": true
        }))
        .await
        .expect("discover should succeed");

    let query_tool = DeviceQueryTool::new(registry);

    let groups = discover_result.data["groups"].as_array().unwrap();

    for group in groups {
        let location = group["name"].as_str().unwrap();
        println!("\n📍 {}:", location);

        if let Some(devices) = group["devices"].as_array() {
            for device in devices {
                let device_id = device["id"].as_str().unwrap();
                let device_name = device["name"].as_str().unwrap();

                // Query detailed data for this device
                let query_result = query_tool
                    .execute(json!({
                        "device_id": device_id,
                        "limit": 10
                    }))
                    .await;

                if let Ok(result) = &query_result {
                    if result.success {
                        let data = &result.data;
                        print!("  - {}: ", device_name);

                        if let Some(metrics) = data["metrics"].as_array() {
                            let values: Vec<String> = metrics
                                .iter()
                                .map(|m| {
                                    let name = m["display_name"].as_str().unwrap();
                                    let value = m["current"].as_f64().unwrap_or(0.0);
                                    let unit = m["unit"].as_str().unwrap();
                                    format!("{} {:.1}{}", name, value, unit)
                                })
                                .collect();

                            println!("{}", values.join(", "));
                        }
                    } else {
                        println!("  - {} (无数据)", device_name);
                    }
                }
            }
        }
    }
}

/// Scenario 8: User asks "Turn off all lights"
///
/// Flow: device.control with multiple targets discovered by name filter
#[tokio::test]
async fn scenario_8_control_by_name_filter() {
    let registry = Arc::new(MockDeviceRegistry::new());

    println!("\n=== 场景8: 关闭所有灯 ===");

    // First discover all lights (devices with name containing "light" or "灯")
    let discover_tool = DeviceDiscoverTool::new(registry.clone());
    let discover_result = discover_tool
        .execute(json!({
            "filter": {"name_contains": "light"}
        }))
        .await
        .expect("discover should succeed");

    println!("发现的灯具:");
    let device_ids: Vec<String> = discover_result.data["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|g| g["devices"].as_array())
        .flatten()
        .map(|d| d["id"].as_str().unwrap().to_string())
        .collect();

    for id in &device_ids {
        println!("  - {}", id);
    }

    // Control all devices
    let control_tool = DeviceControlTool::new(registry);
    let control_result = control_tool
        .execute(json!({
            "device_ids": device_ids,
            "command": "turn_off"
        }))
        .await
        .expect("control should succeed");

    println!("\n控制结果:");
    println!("  总计: {} 个设备", control_result.data["total_targets"]);
    println!("  成功: {} 个", control_result.data["successful"]);
    println!(
        "  确认: {}",
        control_result.data["confirmation"].as_str().unwrap()
    );
}

/// Scenario 9: Analysis workflow - Trend detection
///
/// Flow: device.query with time range → Analyze trend
#[tokio::test]
async fn scenario_9_temperature_trend_analysis() {
    let registry = Arc::new(MockDeviceRegistry::new());
    let query_tool = DeviceQueryTool::new(registry);

    println!("\n=== 场景9: 温度趋势分析 ===");

    let result = query_tool
        .execute(json!({
            "device_id": "sensor_temp_living",
            "metrics": ["temperature"],
            "limit": 24
        }))
        .await
        .expect("query should succeed");

    if let Some(metrics) = result.data["metrics"].as_array() {
        for metric in metrics {
            let name = metric["display_name"].as_str().unwrap();

            println!("\n{} 趋势分析:", name);
            if let Some(current) = metric["current"].as_f64() {
                println!("  当前值: {:.1}°C", current);
            }

            if let Some(stats) = metric["stats"].as_object() {
                println!("  24小时统计:");
                println!("    平均: {:.1}°C", stats["avg"]);
                println!("    范围: {:.1}°C - {:.1}°C", stats["min"], stats["max"]);

                if let Some(trend) = stats["trend"].as_str() {
                    let trend_desc = match trend {
                        "rising" => "上升 ↗",
                        "falling" => "下降 ↘",
                        "stable" => "稳定 →",
                        _ => "未知 ?",
                    };
                    println!("    趋势: {}", trend_desc);
                }
            }

            if let Some(hint) = metric["analysis_hint"].as_str() {
                println!("  💡 {}", hint);
            }
        }
    }
}

/// Helper: Print scenario summary
#[tokio::test]
async fn print_scenario_summary() {
    println!("\n");
    println!("═══════════════════════════════════════════════════════════");
    println!("              NeoMind 核心工具业务场景测试集");
    println!("═══════════════════════════════════════════════════════════");
    println!("\n已实现的核心工具:");
    println!("  1. device.discover  - 设备发现和探索");
    println!("  2. device.query     - 设备数据查询");
    println!("  3. device.control   - 设备控制");
    println!("\n覆盖的业务场景:");
    println!("  ✓ 设备发现和列表展示");
    println!("  ✓ 按位置/类型/状态过滤设备");
    println!("  ✓ 实时数据和历史数据查询");
    println!("  ✓ 单设备和批量设备控制");
    println!("  ✓ 多步骤工作流（发现→查询→控制）");
    println!("  ✓ 趋势分析和异常检测");
    println!("\n═══════════════════════════════════════════════════════════");
}
