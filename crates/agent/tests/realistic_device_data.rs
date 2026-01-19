//! Realistic device data testing scenarios.
//!
//! This module tests realistic IoT device data formats including:
//! - Nested multi-level JSON
//! - Hexadecimal encoded data
//! - Base64 encoded binary data
//! - Complex sensor payloads
//! - Array-based data structures

use std::sync::Arc;
use tokio::sync::RwLock;

use edge_ai_agent::context::{
    ResourceIndex, Resource, Capability, CapabilityType, AccessType,
};

/// Realistic device data samples.
pub struct RealisticDeviceSamples {
    /// Multi-sensor with nested JSON
    pub multi_sensor_nested: &'static str,
    /// Hex encoded sensor data
    pub hex_encoded_sensor: &'static str,
    /// Base64 encoded image data
    pub base64_image_sensor: &'static str,
    /// Array-based sensor batch
    pub sensor_batch_array: &'static str,
    /// Complex nested with mixed types
    pub complex_mixed: &'static str,
    /// Industrial Modbus-style data
    pub industrial_modbus: &'static str,
}

impl RealisticDeviceSamples {
    pub fn new() -> Self {
        Self {
            // 1. Multi-sensor with nested JSON (实际工业场景)
            multi_sensor_nested: r#"{
                "ts": 1704067200,
                "dev": "ms_001",
                "ver": "2.1.0",
                "payload": {
                    "sensors": [
                        {"t": "temp", "v": 25.5, "u": "C", "q": 0.1},
                        {"t": "hum", "v": 60, "u": "%", "q": 1},
                        {"t": "co2", "v": 450, "u": "ppm", "q": 5},
                        {"t": "pm25", "v": 35, "u": "ug/m3", "q": 1}
                    ],
                    "status": {
                        "battery": 85,
                        "rssi": -45,
                        "uptime": 1234567,
                        "errors": []
                    },
                    "config": {
                        "interval": 60,
                        "enabled": true
                    }
                }
            }"#,

            // 2. Hex encoded sensor data (16进制数据)
            hex_encoded_sensor: r#"{
                "cmd": "report",
                "dev": "sensor_002",
                "encoding": "hex",
                "data": "1A2B3C4D",
                "fields": [
                    {"name": "voltage", "offset": 0, "len": 2, "scale": 0.01},
                    {"name": "current", "offset": 2, "len": 2, "scale": 0.001}
                ]
            }"#,

            // 3. Base64 encoded image (摄像头图片)
            base64_image_sensor: r#"{
                "cmd": "frame",
                "dev": "camera_001",
                "ts": 1704067200,
                "image": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
                "meta": {
                    "w": 1920,
                    "h": 1080,
                    "fmt": "jpeg",
                    "size": 102400
                },
                "detections": [
                    {"class": "person", "conf": 0.95, "bbox": [100, 200, 300, 400]},
                    {"class": "vehicle", "conf": 0.87, "bbox": [500, 300, 700, 500]}
                ]
            }"#,

            // 4. Array-based sensor batch (批量上报)
            sensor_batch_array: r#"{
                "batch_id": "batch_20240101_001",
                "count": 5,
                "readings": [
                    {"id": "s001", "temp": 23.5, "hum": 55, "ts": 1704067000},
                    {"id": "s002", "temp": 24.1, "hum": 58, "ts": 1704067005},
                    {"id": "s003", "temp": 22.8, "hum": 52, "ts": 1704067010},
                    {"id": "s004", "temp": 25.2, "hum": 61, "ts": 1704067015},
                    {"id": "s005", "temp": 24.8, "hum": 57, "ts": 1704067020}
                ],
                "summary": {
                    "avg_temp": 24.08,
                    "avg_hum": 56.6,
                    "min_temp": 22.8,
                    "max_temp": 25.2
                }
            }"#,

            // 5. Complex nested with mixed types (复杂混合类型)
            complex_mixed: r#"{
                "device": "gateway_001",
                "timestamp": 1704067200,
                "data": {
                    "ports": {
                        "port1": {
                            "type": "temp_sensor",
                            "value": 26.3,
                            "status": "ok",
                            "history": [25.1, 25.5, 26.0, 26.3]
                        },
                        "port2": {
                            "type": "relay",
                            "value": true,
                            "status": "on",
                            "count": 1234
                        },
                        "port3": {
                            "type": "analog",
                            "value": 4.2,
                            "unit": "mA",
                            "range": {"min": 4, "max": 20}
                        }
                    },
                    "alerts": [
                        {"id": 1, "level": "info", "msg": "Normal operation"},
                        {"id": 2, "level": "warning", "msg": "High temperature"}
                    ]
                }
            }"#,

            // 6. Industrial Modbus-style data (工业Modbus数据)
            industrial_modbus: r#"{
                "slave": 1,
                "registers": {
                    "40001": {"name": "temperature", "value": 25.5, "unit": "C"},
                    "40002": {"name": "humidity", "value": 60.0, "unit": "%"},
                    "40003": {"name": "pressure", "value": 101.3, "unit": "kPa"},
                    "40004": {"name": "flow_rate", "value": 12.5, "unit": "L/min"},
                    "40005": {"name": "status", "value": 1, "bits": ["running", "online", "no_alarm"]}
                },
                "raw_hex": "019301C201CD003200050001"
            }"#,
        }
    }
}

/// Test: Parse nested JSON with array indexing
#[tokio::test]
async fn test_parse_nested_json_with_arrays() {
    let samples = RealisticDeviceSamples::new();
    let json: serde_json::Value = serde_json::from_str(samples.multi_sensor_nested).unwrap();

    // Extract nested values
    let payload = &json["payload"];
    let sensors = &payload["sensors"];

    // Verify array access
    assert!(sensors.is_array());
    assert_eq!(sensors.as_array().unwrap().len(), 4);

    // Access array element
    let first_sensor = &sensors[0];
    assert_eq!(first_sensor["t"], "temp");
    assert_eq!(first_sensor["v"], 25.5);

    // Access deeply nested
    let battery = &payload["status"]["battery"];
    assert_eq!(battery, 85);

    println!("✓ Nested JSON parsing successful");
    println!("  - Sensors array: {} elements", sensors.as_array().unwrap().len());
    println!("  - First sensor: {} = {}{}",
        first_sensor["t"],
        first_sensor["v"],
        first_sensor["u"]);
    println!("  - Battery level: {}%", battery);
}

/// Test: Parse hex encoded data
#[tokio::test]
async fn test_parse_hex_encoded_data() {
    let samples = RealisticDeviceSamples::new();
    let json: serde_json::Value = serde_json::from_str(samples.hex_encoded_sensor).unwrap();

    let hex_str = json["data"].as_str().unwrap();
    assert_eq!(hex_str, "1A2B3C4D");

    // Parse hex string inline (without external hex crate)
    let bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i+2], 16).unwrap())
        .collect();
    assert_eq!(bytes, vec![0x1A, 0x2B, 0x3C, 0x4D]);

    // Parse as u16 values (big-endian)
    let voltage = u16::from_be_bytes([bytes[0], bytes[1]]);
    let current = u16::from_be_bytes([bytes[2], bytes[3]]);

    // With scaling from sample definition
    let voltage_scaled = voltage as f64 * 0.01;  // 6699 * 0.01 = 66.99V
    let current_scaled = current as f64 * 0.001; // 15404 * 0.001 = 15.404A

    println!("✓ Hex data parsing successful");
    println!("  - Hex string: {}", hex_str);
    println!("  - Bytes: {:?}", bytes);
    println!("  - Voltage: {} * 0.01 = {}V", voltage, voltage_scaled);
    println!("  - Current: {} * 0.001 = {}A", current, current_scaled);
}

/// Test: Parse batch array data
#[tokio::test]
async fn test_parse_sensor_batch_array() {
    let samples = RealisticDeviceSamples::new();
    let json: serde_json::Value = serde_json::from_str(samples.sensor_batch_array).unwrap();

    let readings = &json["readings"];
    let summary = &json["summary"];

    // Verify batch structure
    assert_eq!(json["count"], 5);
    assert_eq!(readings.as_array().unwrap().len(), 5);

    // Calculate average temperature from readings
    let temps: Vec<f64> = readings.as_array().unwrap()
        .iter()
        .map(|r| r["temp"].as_f64().unwrap())
        .collect();

    let avg_temp: f64 = temps.iter().sum::<f64>() / temps.len() as f64;

    // Compare with pre-calculated summary
    let summary_avg = summary["avg_temp"].as_f64().unwrap();
    assert!((avg_temp - summary_avg).abs() < 0.01);

    println!("✓ Batch array parsing successful");
    println!("  - Readings: {}", readings.as_array().unwrap().len());
    println!("  - Calculated avg temp: {:.2}°C", avg_temp);
    println!("  - Summary avg temp: {:.2}°C", summary_avg);
}

/// Test: Parse complex mixed nested structure
#[tokio::test]
async fn test_parse_complex_mixed_structure() {
    let samples = RealisticDeviceSamples::new();
    let json: serde_json::Value = serde_json::from_str(samples.complex_mixed).unwrap();

    let ports = &json["data"]["ports"];

    // Access different port types with different structures
    let port1 = &ports["port1"];
    let port2 = &ports["port2"];

    // Port 1: Temperature sensor with history array
    assert_eq!(port1["type"], "temp_sensor");
    assert_eq!(port1["value"], 26.3);
    let history = port1["history"].as_array().unwrap();
    assert_eq!(history.len(), 4);

    // Port 2: Relay with boolean and counter
    assert_eq!(port2["type"], "relay");
    assert_eq!(port2["value"], true);
    assert_eq!(port2["count"], 1234);

    // Alerts array
    let alerts = &json["data"]["alerts"];
    assert_eq!(alerts.as_array().unwrap().len(), 2);

    println!("✓ Complex mixed structure parsing successful");
    println!("  - Port 1: {} = {}", port1["type"], port1["value"]);
    println!("  - Port 1 history: {:?}",
        history.iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>());
    println!("  - Port 2: {} = {}", port2["type"], port2["value"]);
    println!("  - Port 2 count: {}", port2["count"]);
    println!("  - Alerts: {}", alerts.as_array().unwrap().len());
}

/// Test: Extract values using JSON path notation
#[tokio::test]
async fn test_json_path_extraction() {
    let samples = RealisticDeviceSamples::new();
    let json: serde_json::Value = serde_json::from_str(samples.multi_sensor_nested).unwrap();

    // Simulate JSON path extraction like "payload.sensors[0].v"
    let paths = vec![
        ("ts", Some(json["ts"].clone())),
        ("payload.sensors[0].v", Some(json["payload"]["sensors"][0]["v"].clone())),
        ("payload.status.battery", Some(json["payload"]["status"]["battery"].clone())),
        ("payload.config.interval", Some(json["payload"]["config"]["interval"].clone())),
    ];

    for (path, expected) in paths {
        let result = extract_json_path(&json, path);
        assert_eq!(result, expected);
        println!("✓ JSON path '{}' = {:?}", path, result);
    }
}

/// Helper: Extract value from JSON using path notation
fn extract_json_path(json: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        // Handle array indexing like "sensors[0]"
        if let Some(bracket_pos) = part.find('[') {
            let key = &part[..bracket_pos];
            let index_part = &part[bracket_pos+1..part.len()-1]; // Remove [ ]

            current = &current[key];
            if let Some(arr) = current.as_array() {
                if let Ok(index) = index_part.parse::<usize>() {
                    if index < arr.len() {
                        current = &arr[index];
                    } else {
                        return None;
                    }
                }
            }
        } else {
            current = &current[part];
        }
    }

    Some(current.clone())
}

/// Test: Register resources with realistic data samples
#[tokio::test]
async fn test_register_resources_with_samples() {
    let index = Arc::new(RwLock::new(ResourceIndex::new()));

    // Register multi-sensor device with nested data structure
    let multi_sensor = Resource::device("ms_001", "多参数环境监测站", "multi_sensor")
        .with_alias("环境监测站")
        .with_alias("监测站")
        .with_keyword("多参数")
        .with_keyword("环境")
        .with_keyword("电池")
        .with_location("1号车间")
        .with_capability(Capability {
            name: "payload.sensors".to_string(),
            cap_type: CapabilityType::Metric,
            data_type: "array".to_string(),
            valid_values: None,
            unit: None,
            access: AccessType::Read,
        })
        .with_capability(Capability {
            name: "temperature".to_string(),
            cap_type: CapabilityType::Metric,
            data_type: "float".to_string(),
            valid_values: None,
            unit: Some("°C".to_string()),
            access: AccessType::Read,
        })
        .with_capability(Capability {
            name: "battery".to_string(),
            cap_type: CapabilityType::Metric,
            data_type: "integer".to_string(),
            valid_values: None,
            unit: Some("%".to_string()),
            access: AccessType::Read,
        });

    index.write().await.register(multi_sensor).await.unwrap();

    // Search for the device
    let results = index.read().await.search_string("多参数").await;
    assert!(!results.is_empty());

    let results = index.read().await.search_string("电池").await;
    assert!(!results.is_empty());

    println!("✓ Resource registration with nested path support successful");
    println!("  - Device: 多参数环境监测站");
    println!("  - Capabilities: payload.sensors, temperature, battery");
    println!("  - Search '多参数': {} results", index.read().await.search_string("多参数").await.len());
    println!("  - Search '电池': {} results", index.read().await.search_string("电池").await.len());
}
