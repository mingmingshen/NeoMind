//! Mock device data for testing the resource index.
//!
//! This module provides 30 simulated devices covering common IoT scenarios:
//! - Environmental sensors (temperature, humidity, pressure, air quality)
//! - Lighting controls
//! - HVAC (heating, ventilation, air conditioning)
//! - Security sensors and cameras
//! - Appliances (smart plugs, smart locks)
//! - Industrial sensors
//! - Energy monitoring

use crate::context::{Resource, Capability, CapabilityType, AccessType};

/// Generate 30 mock devices for testing.
pub fn generate_mock_devices() -> Vec<Resource> {
    vec![
        // === 客厅 (Living Room) - 7 devices ===
        Resource::device("living_temp_sensor", "客厅温度传感器", "dht22_sensor")
            .with_alias("室温传感器").with_alias("客厅温度")
            .with_location("客厅")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            })
            .with_capability(Capability {
                name: "humidity".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("living_humidity_sensor", "客厅湿度传感器", "dht22_sensor")
            .with_alias("湿度传感器").with_alias("客厅湿度")
            .with_location("客厅")
            .with_capability(Capability {
                name: "humidity".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("living_light_main", "客厅主灯", "switch_dimmer")
            .with_alias("主灯").with_alias("客厅灯")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "brightness".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::ReadWrite,
            }),

        Resource::device("living_lightAccent", "客厅氛围灯", "rgb_light")
            .with_alias("氛围灯").with_alias("彩灯")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "color".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "string".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            }),

        Resource::device("living_ac", "客厅空调", "ac_unit")
            .with_alias("空调").with_alias("AC")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Write,
            })
            .with_capability(Capability {
                name: "mode".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "string".to_string(),
                valid_values: Some(vec!["cool".to_string(), "heat".to_string(), "fan".to_string(), "auto".to_string()]),
                unit: None,
                access: AccessType::Write,
            }),

        Resource::device("living_air_purifier", "客厅空气净化器", "air_purifier")
            .with_alias("净化器").with_alias("空气净化")
            .with_location("客厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "aqi".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "int".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Read,
            }),

        Resource::device("living_curtain", "客厅窗帘", "curtain_motor")
            .with_alias("窗帘").with_alias("电动窗帘")
            .with_location("客厅")
            .with_capability(Capability {
                name: "position".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::ReadWrite,
            }),

        // === 卧室 (Bedroom) - 6 devices ===
        Resource::device("bedroom_temp_sensor", "卧室温度传感器", "dht22_sensor")
            .with_alias("卧室温度").with_alias("室温")
            .with_location("卧室")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("bedroom_light", "卧室灯", "switch_dimmer")
            .with_alias("床头灯")
            .with_location("卧室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "brightness".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::ReadWrite,
            }),

        Resource::device("bedroom_ac", "卧室空调", "ac_unit")
            .with_alias("卧室AC")
            .with_location("卧室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Write,
            }),

        Resource::device("bedroom_curtain", "卧室窗帘", "curtain_motor")
            .with_alias("窗帘")
            .with_location("卧室")
            .with_capability(Capability {
                name: "position".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::ReadWrite,
            }),

        Resource::device("bedroom_co2_sensor", "卧室二氧化碳传感器", "co2_sensor")
            .with_alias("CO2传感器").with_alias("空气质量")
            .with_location("卧室")
            .with_capability(Capability {
                name: "co2".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("ppm".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("bedroom_plug", "卧室插座", "smart_plug")
            .with_alias("智能插座")
            .with_location("卧室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "energy".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("kWh".to_string()),
                access: AccessType::Read,
            }),

        // === 厨房 (Kitchen) - 5 devices ===
        Resource::device("kitchen_temp_sensor", "厨房温度传感器", "temp_sensor")
            .with_alias("厨房温度")
            .with_location("厨房")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("kitchen_smoke_detector", "厨房烟雾报警器", "smoke_detector")
            .with_alias("烟雾传感器").with_alias("火灾报警")
            .with_location("厨房")
            .with_capability(Capability {
                name: "smoke_level".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "int".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Read,
            }),

        Resource::device("kitchen_gas_detector", "厨房燃气报警器", "gas_detector")
            .with_alias("燃气传感器").with_alias("甲烷传感器")
            .with_location("厨房")
            .with_capability(Capability {
                name: "gas_level".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("LEL".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("kitchen_light", "厨房灯", "switch")
            .with_location("厨房")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            }),

        Resource::device("kitchen_water_leak", "厨房漏水检测", "water_leak_sensor")
            .with_alias("漏水传感器")
            .with_location("厨房")
            .with_capability(Capability {
                name: "leak_detected".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Read,
            }),

        // === 浴室 (Bathroom) - 4 devices ===
        Resource::device("bathroom_temp_sensor", "浴室温度传感器", "temp_sensor")
            .with_alias("浴室温度")
            .with_location("浴室")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("bathroom_humidity_sensor", "浴室湿度传感器", "humidity_sensor")
            .with_alias("浴室湿度")
            .with_location("浴室")
            .with_capability(Capability {
                name: "humidity".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("bathroom_light", "浴室灯", "switch")
            .with_location("浴室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            }),

        Resource::device("bathroom_heater", "浴室浴霸", "bathroom_heater")
            .with_alias("浴霸").with_alias("取暖器")
            .with_location("浴室")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "level".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "int".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Write,
            }),

        // === 阳台 (Balcony) - 3 devices ===
        Resource::device("balcony_temp_sensor", "阳台温度传感器", "temp_sensor")
            .with_alias("阳台温度")
            .with_location("阳台")
            .with_capability(Capability {
                name: "temperature".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("°C".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("balcony_light_sensor", "阳台光照传感器", "light_sensor")
            .with_alias("光照传感器").with_alias("亮度传感器")
            .with_location("阳台")
            .with_capability(Capability {
                name: "illuminance".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "float".to_string(),
                valid_values: None,
                unit: Some("lux".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("balcony_rain_sensor", "阳台雨滴传感器", "rain_sensor")
            .with_alias("雨滴传感器").with_alias("下雨检测")
            .with_location("阳台")
            .with_capability(Capability {
                name: "raining".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Read,
            }),

        // === 门厅 (Entrance) - 3 devices ===
        Resource::device("entrance_light", "门厅灯", "switch")
            .with_alias("玄关灯")
            .with_location("门厅")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            }),

        Resource::device("door_lock", "智能门锁", "smart_lock")
            .with_alias("门锁").with_alias("智能锁")
            .with_location("门厅")
            .with_capability(Capability {
                name: "locked".to_string(),
                cap_type: CapabilityType::Property,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            })
            .with_capability(Capability {
                name: "battery".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "int".to_string(),
                valid_values: None,
                unit: Some("%".to_string()),
                access: AccessType::Read,
            }),

        Resource::device("doorbell_camera", "门铃摄像头", "doorbell_camera")
            .with_alias("可视门铃").with_alias("监控")
            .with_location("门厅")
            .with_capability(Capability {
                name: "motion_detected".to_string(),
                cap_type: CapabilityType::Metric,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::Read,
            }),

        // === 车库 (Garage) - 2 devices ===
        Resource::device("garage_door", "车库门", "garage_door")
            .with_alias("卷帘门")
            .with_location("车库")
            .with_capability(Capability {
                name: "position".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "string".to_string(),
                valid_values: Some(vec!["open".to_string(), "closed".to_string(), "stopped".to_string()]),
                unit: None,
                access: AccessType::ReadWrite,
            }),

        Resource::device("garage_light", "车库灯", "switch")
            .with_location("车库")
            .with_capability(Capability {
                name: "power".to_string(),
                cap_type: CapabilityType::Command,
                data_type: "bool".to_string(),
                valid_values: None,
                unit: None,
                access: AccessType::ReadWrite,
            }),
    ]
}

/// Generate 300 mock devices for large-scale testing.
///
/// This creates devices across multiple floors, rooms, and types:
/// - 10 floors (1楼-10楼)
/// - 8 rooms per floor (客厅, 卧室, 厨房, 浴室, 书房, 阳台, 餐厅, 走廊)
/// - Multiple device types per room
pub fn generate_large_scale_devices(count: usize) -> Vec<Resource> {
    let floors = &["1楼", "2楼", "3楼", "4楼", "5楼", "6楼", "7楼", "8楼", "9楼", "10楼"];
    let rooms = &["客厅", "卧室", "厨房", "浴室", "书房", "阳台", "餐厅", "走廊"];

    let mut devices = Vec::new();
    let mut device_id = 0;

    // Device templates that will be instantiated for each room
    let device_templates: Vec<(&str, &str, Vec<(&str, CapabilityType, AccessType, Option<&str>)>)> = vec![
        // Temperature sensor
        ("temp_sensor", "温度传感器", vec![
            ("temperature", CapabilityType::Metric, AccessType::Read, Some("°C")),
        ]),
        // Humidity sensor
        ("humidity_sensor", "湿度传感器", vec![
            ("humidity", CapabilityType::Metric, AccessType::Read, Some("%")),
        ]),
        // Light (with dimmer)
        ("light_dimmer", "灯", vec![
            ("power", CapabilityType::Command, AccessType::ReadWrite, None),
            ("brightness", CapabilityType::Command, AccessType::ReadWrite, Some("%")),
        ]),
        // Simple switch
        ("switch", "开关", vec![
            ("power", CapabilityType::Command, AccessType::ReadWrite, None),
        ]),
        // AC unit
        ("ac_unit", "空调", vec![
            ("power", CapabilityType::Command, AccessType::ReadWrite, None),
            ("temperature", CapabilityType::Command, AccessType::Write, Some("°C")),
            ("mode", CapabilityType::Command, AccessType::Write, None),
        ]),
        // Curtain
        ("curtain", "窗帘", vec![
            ("position", CapabilityType::Command, AccessType::ReadWrite, Some("%")),
        ]),
        // Smart plug
        ("smart_plug", "插座", vec![
            ("power", CapabilityType::Command, AccessType::ReadWrite, None),
            ("energy", CapabilityType::Metric, AccessType::Read, Some("kWh")),
        ]),
        // Motion sensor
        ("motion_sensor", "人体传感器", vec![
            ("motion", CapabilityType::Metric, AccessType::Read, None),
        ]),
        // Door/window sensor
        ("door_sensor", "门窗传感器", vec![
            ("open", CapabilityType::Metric, AccessType::Read, None),
        ]),
        // CO2 sensor
        ("co2_sensor", "二氧化碳传感器", vec![
            ("co2", CapabilityType::Metric, AccessType::Read, Some("ppm")),
        ]),
        // PM2.5 sensor
        ("pm25_sensor", "PM2.5传感器", vec![
            ("pm25", CapabilityType::Metric, AccessType::Read, Some("µg/m³")),
        ]),
        // Light sensor
        ("light_sensor", "光照传感器", vec![
            ("illuminance", CapabilityType::Metric, AccessType::Read, Some("lux")),
        ]),
    ];

    for (floor_idx, floor) in floors.iter().enumerate() {
        for (room_idx, room) in rooms.iter().enumerate() {
            let location = format!("{}{}", floor, room);

            // Assign different devices to different rooms to add variety
            let room_hash = (floor_idx * 10 + room_idx) % device_templates.len();

            // Add 2-4 devices per room depending on room type
            let devices_in_room = match *room {
                "客厅" | "卧室" => 4,
                "厨房" | "浴室" => 3,
                _ => 2,
            };

            for i in 0..devices_in_room {
                if device_id >= count {
                    break;
                }

                let template_idx = (room_hash + i) % device_templates.len();
                let (device_type, device_name_suffix, capabilities) = &device_templates[template_idx];

                let id = format!("{}_{}_{}", floor, room, device_type);
                let name = format!("{}{}", location, device_name_suffix);

                let mut resource = Resource::device(id.clone(), name.clone(), *device_type)
                    .with_location(location.as_str());

                // Add capabilities
                for (cap_name, cap_type, access, unit) in capabilities {
                    resource = resource.with_capability(Capability {
                        name: cap_name.to_string(),
                        cap_type: cap_type.clone(),
                        data_type: match cap_type {
                            CapabilityType::Metric => "float".to_string(),
                            CapabilityType::Command => match *cap_name {
                                "power" | "open" | "motion" => "bool".to_string(),
                                "mode" => "string".to_string(),
                                _ => "float".to_string(),
                            },
                            CapabilityType::Property => "float".to_string(),
                        },
                        valid_values: None,
                        unit: unit.map(|u| u.to_string()),
                        access: access.clone(),
                    });
                }

                // Add some aliases for common devices
                match *device_type {
                    "light_dimmer" | "switch" => {
                        resource = resource.with_alias("灯").with_alias("开关");
                    }
                    "ac_unit" => {
                        resource = resource.with_alias("空调").with_alias("AC");
                    }
                    "temp_sensor" => {
                        resource = resource.with_alias("温度").with_alias("温度计");
                    }
                    "humidity_sensor" => {
                        resource = resource.with_alias("湿度").with_alias("湿度计");
                    }
                    "curtain" => {
                        resource = resource.with_alias("窗帘").with_alias("电动窗帘");
                    }
                    _ => {}
                }

                devices.push(resource);
                device_id += 1;
            }

            if device_id >= count {
                break;
            }
        }

        if device_id >= count {
            break;
        }
    }

    // Ensure we have exactly the requested count
    while devices.len() < count {
        let idx = devices.len();
        devices.push(Resource::device(
            format!("extra_device_{}", idx),
            format!("额外设备{}", idx),
            "generic"
        ).with_capability(Capability {
            name: "status".to_string(),
            cap_type: CapabilityType::Metric,
            data_type: "string".to_string(),
            valid_values: None,
            unit: None,
            access: AccessType::Read,
        }));
    }

    devices.truncate(count);
    devices
}

/// Get a brief summary of all mock devices.
pub fn get_device_summary() -> String {
    let devices = generate_mock_devices();

    let mut summary = String::from("模拟设备列表 (30个设备)\n\n");

    // Group by location
    let mut by_location: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();

    for device in &devices {
        let location = device.as_device()
            .and_then(|d| d.location.as_deref())
            .unwrap_or("其他");

        by_location.entry(location).or_default().push(device.name.as_str());
    }

    // Format by location
    for (location, device_names) in [
        ("客厅", "7个设备"),
        ("卧室", "6个设备"),
        ("厨房", "5个设备"),
        ("浴室", "4个设备"),
        ("阳台", "3个设备"),
        ("门厅", "3个设备"),
        ("车库", "2个设备"),
    ] {
        if let Some(devs) = by_location.get(location) {
            summary.push_str(&format!("**{}** ({})\n", location, device_names.len()));
            for name in devs {
                summary.push_str(&format!("  - {}\n", name));
            }
            summary.push('\n');
        }
    }

    summary.push_str(&format!("总计: {} 个设备\n", devices.len()));

    // Capability summary
    let mut metrics = 0;
    let mut commands = 0;
    for device in &devices {
        if let Some(d) = device.as_device() {
            for cap in &d.capabilities {
                if cap.cap_type == CapabilityType::Metric || cap.access == AccessType::Read {
                    metrics += 1;
                }
                if cap.cap_type == CapabilityType::Command || cap.access == AccessType::Write {
                    commands += 1;
                }
            }
        }
    }

    summary.push_str(&format!("可查询指标: {} 个\n", metrics));
    summary.push_str(&format!("可执行命令: {} 个\n", commands));

    summary
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::context::{ResourceIndex, ResourceResolver, DynamicToolGenerator, Capability, CapabilityType, AccessType};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Integration test: Register 30 mock devices and test various queries.
    #[tokio::test]
    async fn test_30_mock_devices_integration() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        // Register all 30 mock devices
        let devices = generate_mock_devices();
        assert_eq!(devices.len(), 30, "Should have 30 mock devices");

        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let resolver = ResourceResolver::new(Arc::clone(&index));

        // Test 1: Query temperature in living room
        let resolved = resolver.resolve("客厅温度是多少").await;
        assert_eq!(resolved.intent, crate::context::resource_resolver::IntentCategory::QueryData);
        assert!(!resolved.resources.is_empty(), "Should find temperature devices in living room");

        // Test 2: Control living room light
        let resolved = resolver.resolve("打开客厅灯").await;
        assert_eq!(resolved.intent, crate::context::resource_resolver::IntentCategory::ControlDevice);
        assert!(!resolved.actions.is_empty(), "Should generate action for controlling living room light");

        // Test 3: List all devices
        let resolved = resolver.resolve("列出所有设备").await;
        assert_eq!(resolved.intent, crate::context::resource_resolver::IntentCategory::ListDevices);

        // Test 4: Query by location
        let resolved = resolver.resolve("卧室有什么设备").await;
        assert!(!resolved.resources.is_empty(), "Should find devices in bedroom");

        // Test 5: Fuzzy search by alias
        let resolved = resolver.resolve("打开主灯").await;
        assert!(!resolved.resources.is_empty(), "Should find living room main light by alias");

        // Test 6: Query by capability
        let resolved = resolver.resolve("有哪些温度传感器").await;
        assert!(!resolved.resources.is_empty(), "Should find temperature sensors");

        // Test 7: Ambiguous query should trigger clarification
        let resolved = resolver.resolve("温度是多少").await;
        assert!(resolved.clarification.is_some() || !resolved.resources.is_empty(),
            "Ambiguous query should either have clarification or find multiple devices");

        // Test 8: Control device in bedroom
        let resolved = resolver.resolve("关闭卧室空调").await;
        assert_eq!(resolved.intent, crate::context::resource_resolver::IntentCategory::ControlDevice);
        assert!(!resolved.actions.is_empty(), "Should generate action for controlling bedroom AC");

        // Test 9: Query humidity
        let resolved = resolver.resolve("客厅湿度").await;
        assert!(!resolved.resources.is_empty(), "Should find humidity sensor in living room");

        // Test 10: Device summary
        let stats = index.read().await.stats().await;
        assert_eq!(stats.total_resources, 30, "Should have 30 resources registered");
    }

    /// Test dynamic tool generation with 30 devices.
    #[tokio::test]
    async fn test_dynamic_tools_with_30_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        // Register all 30 mock devices
        let devices = generate_mock_devices();
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let generator = DynamicToolGenerator::new(Arc::clone(&index));

        // Generate tools
        let tools = generator.generate_tools().await;

        // Should have at least the discovery tools + device tools
        assert!(tools.len() >= 5, "Should have at least 5 tools");

        // Verify tool names
        let tool_names: Vec<_> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"search_resources".to_string()), "Should have search_resources tool");
        assert!(tool_names.contains(&"list_devices".to_string()), "Should have list_devices tool");
        assert!(tool_names.contains(&"query_data".to_string()), "Should have query_data tool");
        assert!(tool_names.contains(&"control_device".to_string()), "Should have control_device tool");

        // Verify list_devices tool has device context
        let list_devices = tools.iter().find(|t| t.name == "list_devices").unwrap();
        assert!(list_devices.description.contains("30"), "Should mention 30 devices");
    }

    /// Test search performance with 30 devices.
    #[tokio::test]
    async fn test_search_performance_with_30_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        // Register all 30 mock devices
        let devices = generate_mock_devices();
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        // Test various search queries
        let queries = vec![
            "客厅温度",
            "打开灯",
            "卧室空调",
            "温度传感器",
            "烟雾报警",
            "门锁状态",
        ];

        for query in queries {
            let start = std::time::Instant::now();
            let results = index.read().await.search_string(query).await;
            let elapsed = start.elapsed();

            // Search should be fast (< 10ms)
            assert!(elapsed.as_millis() < 10, "Search for '{}' should be fast, took {:?}", query, elapsed);

            // Most queries should find something
            if query != "打开灯" {  // This one is ambiguous
                assert!(!results.is_empty(), "Query '{}' should find results", query);
            }
        }
    }

    /// Test device summary generation.
    #[tokio::test]
    async fn test_device_summary_generation() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        // Register all 30 mock devices
        let devices = generate_mock_devices();
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let generator = DynamicToolGenerator::new(index);

        let summary = generator.device_summary().await;

        // Verify summary contains expected content
        assert!(summary.contains("系统设备"), "Should have system devices header");
        assert!(summary.contains("客厅"), "Should mention living room");
        assert!(summary.contains("卧室"), "Should mention bedroom");
        assert!(summary.contains("厨房"), "Should mention kitchen");
    }
}

#[cfg(test)]
mod large_scale_tests {
    use super::*;
    use crate::context::{ResourceIndex, ResourceResolver, DynamicToolGenerator};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Test generating 300 devices.
    #[tokio::test]
    async fn test_generate_300_devices() {
        let devices = generate_large_scale_devices(300);
        assert_eq!(devices.len(), 300, "Should generate exactly 300 devices");

        // Count devices by type
        let mut type_counts = std::collections::HashMap::new();
        for device in &devices {
            if let Some(dev_data) = device.as_device() {
                *type_counts.entry(dev_data.device_type.clone()).or_insert(0) += 1;
            }
        }

        println!("Device type distribution:");
        for (type_name, count) in type_counts.iter() {
            println!("  {}: {}", type_name, count);
        }

        // Should have multiple types
        assert!(type_counts.len() >= 5, "Should have at least 5 different device types");

        // Each device should have at least one capability
        for device in &devices {
            if let Some(dev_data) = device.as_device() {
                assert!(!dev_data.capabilities.is_empty(),
                    "Device {} should have capabilities", device.name);
            }
        }
    }

    /// Test search performance with 300 devices.
    #[tokio::test]
    async fn test_search_performance_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let resolver = ResourceResolver::new(Arc::clone(&index));

        // Test various queries and measure performance
        let test_queries = vec![
            "1楼客厅温度是多少",
            "打开3楼卧室的灯",
            "5楼有哪些温度传感器",
            "2楼厨房的空调关闭",
            "10楼客厅湿度",
            "打开所有房间的灯",
            "7楼书房温度",
            "4楼阳台光照强度",
            "有哪些门窗传感器",
            "8楼餐厅设备列表",
        ];

        let mut total_time = std::time::Duration::from_secs(0);
        let mut max_time = std::time::Duration::from_secs(0);

        for query in &test_queries {
            let start = std::time::Instant::now();
            let resolved = resolver.resolve(query).await;
            let elapsed = start.elapsed();

            total_time += elapsed;
            max_time = max_time.max(elapsed);

            // All queries should get some results or clarification
            let has_results = !resolved.resources.is_empty();
            let has_clarification = resolved.clarification.is_some();
            assert!(has_results || has_clarification,
                "Query '{}' should have results or clarification", query);
        }

        let avg_time = total_time / test_queries.len() as u32;

        println!("Search performance with 300 devices:");
        println!("  Average: {:?}", avg_time);
        println!("  Max: {:?}", max_time);

        // Performance assertions
        assert!(max_time.as_millis() < 50, "Max search time should be under 50ms, got {:?}", max_time);
        assert!(avg_time.as_millis() < 20, "Average search time should be under 20ms, got {:?}", avg_time);
    }

    /// Test dynamic tool generation with 300 devices.
    #[tokio::test]
    async fn test_dynamic_tools_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let generator = DynamicToolGenerator::new(Arc::clone(&index));

        let tools = generator.generate_tools().await;

        // Verify core tools exist
        let tool_names: Vec<_> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"search_resources".to_string()));
        assert!(tool_names.contains(&"list_devices".to_string()));
        assert!(tool_names.contains(&"query_data".to_string()));
        assert!(tool_names.contains(&"control_device".to_string()));

        // list_devices tool should mention 300 devices
        let list_devices = tools.iter().find(|t| t.name == "list_devices").unwrap();
        assert!(list_devices.description.contains("300"),
            "Should mention 300 devices, got: {}", list_devices.description);

        println!("Generated {} tools from 300 devices", tools.len());
    }

    /// Simulate LLM conversation with 300 devices.
    #[tokio::test]
    async fn test_llm_conversation_simulation_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let resolver = ResourceResolver::new(Arc::clone(&index));
        let generator = DynamicToolGenerator::new(Arc::clone(&index));

        // Simulate a multi-turn conversation
        let conversation = vec![
            ("你好", "greeting"),
            ("有哪些设备", "list_all"),
            ("1楼客厅温度是多少", "query_temp_1f_living"),
            ("打开3楼卧室的灯", "control_light_3f_bedroom"),
            ("2楼厨房的空调关掉", "control_ac_2f_kitchen"),
            ("5楼有哪些温度传感器", "list_temp_sensors_5f"),
            ("关闭所有房间的灯", "control_all_lights"),
            ("7楼书房湿度怎么样", "query_humidity_7f_study"),
            ("8楼和9楼各有哪些设备", "list_devices_8f_9f"),
            ("10楼客厅的空调温度调到26度", "set_ac_temp_10f_living"),
        ];

        let mut successful_turns = 0;
        let total_turns = conversation.len();

        for (query, scenario) in conversation {
            let start = std::time::Instant::now();
            let resolved = resolver.resolve(query).await;
            let elapsed = start.elapsed();

            // Get relevant tools for this query
            let tools = generator.generate_tools_for_query(query).await;

            // Verify response quality
            let has_intent = matches!(resolved.intent,
                crate::context::resource_resolver::IntentCategory::ListDevices |
                crate::context::resource_resolver::IntentCategory::QueryData |
                crate::context::resource_resolver::IntentCategory::ControlDevice |
                crate::context::resource_resolver::IntentCategory::SystemStatus |
                crate::context::resource_resolver::IntentCategory::General);

            let has_response = !resolved.actions.is_empty()
                || resolved.clarification.is_some()
                || !resolved.resources.is_empty();

            if has_intent && has_response && elapsed.as_millis() < 100 {
                successful_turns += 1;
                println!("✓ [{}] '{}' -> intent={:?}, resources={}, actions={}, tools={}, time={:?}",
                    scenario, query, resolved.intent, resolved.resources.len(),
                    resolved.actions.len(), tools.len(), elapsed);
            } else {
                println!("✗ [{}] '{}' -> intent={:?}, resources={}, actions={}, time={:?}",
                    scenario, query, resolved.intent, resolved.resources.len(),
                    resolved.actions.len(), elapsed);
            }
        }

        let success_rate = (successful_turns as f32 / total_turns as f32) * 100.0;
        println!("\nConversation success rate: {:.1}% ({}/{})",
            success_rate, successful_turns, total_turns);

        assert!(success_rate >= 80.0,
            "Success rate should be at least 80%, got {:.1}%", success_rate);
    }

    /// Test resource distribution across floors with 300 devices.
    #[tokio::test]
    async fn test_resource_distribution_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let stats = index.read().await.stats().await;
        assert_eq!(stats.total_resources, 300, "Should have 300 resources");

        // Count by floor using list_by_type
        let mut floor_counts = std::collections::HashMap::new();
        let devices = index.read().await.list_by_type("device").await;
        for resource in devices {
            if let Some(dev_data) = resource.as_device() {
                if let Some(location) = &dev_data.location {
                    // Extract floor (e.g., "1楼客厅" -> "1楼")
                    let floor: String = location.chars().take(2).collect();
                    *floor_counts.entry(floor).or_insert(0) += 1;
                }
            }
        }

        println!("Device distribution by floor:");
        for floor in ["1楼", "2楼", "3楼", "4楼", "5楼", "6楼", "7楼", "8楼", "9楼", "10楼"] {
            let count = floor_counts.get(floor).unwrap_or(&0);
            println!("  {}: {} devices", floor, count);
        }

        // Each floor should have at least some devices
        for floor in ["1楼", "2楼", "3楼", "4楼", "5楼"] {
            let count = floor_counts.get(floor).unwrap_or(&0);
            assert!(count >= &20, "Floor {} should have at least 20 devices, got {}", floor, count);
        }
    }

    /// Test ambiguous query handling with 300 devices.
    #[tokio::test]
    async fn test_ambiguous_queries_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let resolver = ResourceResolver::new(index);

        // These queries are ambiguous and should trigger clarification
        let ambiguous_queries = vec![
            "温度是多少",  // Which floor? Which room?
            "打开灯",      // Which floor? Which room?
            "空调状态",    // Which AC?
        ];

        for query in ambiguous_queries {
            let resolved = resolver.resolve(query).await;

            // Should either have clarification or find many devices
            let has_clarification = resolved.clarification.is_some();
            let has_many_results = resolved.resources.len() > 10;

            println!("Ambiguous query '{}': clarification={}, results={}",
                query, has_clarification, resolved.resources.len());

            assert!(has_clarification || has_many_results,
                "Ambiguous query '{}' should have clarification or many results", query);
        }
    }

    /// Test precise queries with 300 devices.
    #[tokio::test]
    async fn test_precise_queries_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        let resolver = ResourceResolver::new(index);

        // These queries are specific and should find exact matches
        let precise_queries = vec![
            ("1楼客厅温度是多少", "query_temp"),
            ("打开3楼卧室灯", "control_light"),
            ("关闭5楼厨房空调", "control_ac"),
        ];

        for (query, _scenario) in precise_queries {
            let resolved = resolver.resolve(query).await;

            println!("Precise query '{}': intent={:?}, resources={}, confidence={:.2}",
                query, resolved.intent, resolved.resources.len(), resolved.confidence);

            // Should have high confidence and some resources
            assert!(resolved.confidence >= 0.5,
                "Precise query should have confidence >= 0.5, got {:.2}", resolved.confidence);
            assert!(!resolved.resources.is_empty(),
                "Precise query should find resources");
        }
    }

    /// Stress test: Register 300 devices and verify all are indexed.
    #[tokio::test]
    async fn test_registration_stress_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);

        let start = std::time::Instant::now();
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }
        let registration_time = start.elapsed();

        let stats = index.read().await.stats().await;

        println!("Registration of 300 devices: {:?}", registration_time);
        println!("Total resources: {}", stats.total_resources);

        assert_eq!(stats.total_resources, 300, "All 300 devices should be registered");
        assert!(registration_time.as_millis() < 500,
            "Registration should complete in under 500ms, got {:?}", registration_time);
    }

    /// Test memory efficiency with 300 devices.
    #[tokio::test]
    async fn test_memory_efficiency_300_devices() {
        let index = Arc::new(RwLock::new(ResourceIndex::new()));

        let devices = generate_large_scale_devices(300);
        for device in devices {
            index.write().await.register(device).await.unwrap();
        }

        // Generate tools multiple times to test caching
        let generator = DynamicToolGenerator::new(Arc::clone(&index));

        let start = std::time::Instant::now();
        let tools1 = generator.generate_tools().await;
        let first_generation = start.elapsed();

        let start = std::time::Instant::now();
        let tools2 = generator.generate_tools().await;  // Should use cache
        let cached_generation = start.elapsed();

        println!("Tool generation (300 devices):");
        println!("  First: {:?}", first_generation);
        println!("  Cached: {:?}", cached_generation);

        // Cached generation should be much faster
        assert!(cached_generation < first_generation,
            "Cached generation should be faster than first");

        // Results should be identical
        assert_eq!(tools1.len(), tools2.len(), "Cached tools should match");
    }
}
