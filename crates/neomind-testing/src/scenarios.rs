//! Test scenario builder
//!
//! Provides a fluent API for building complex test scenarios
//! for AI Agent testing.

use super::device_simulator::{DeviceSimulator, SimulatedDevice, SimulatedDeviceType};
use super::test_data::TestDataGenerator;
use serde::{Deserialize, Serialize};

/// A test scenario with devices and expected behaviors
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub devices: Vec<SimulatedDevice>,
    pub time_patterns: Vec<TimePattern>,
    pub expected_behaviors: Vec<ExpectedBehavior>,
}

/// Time-based pattern for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePattern {
    pub pattern_type: TimePatternType,
    pub schedule: String,
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePatternType {
    Daily,
    Weekly,
    Interval,
    EventDriven,
}

/// Expected behavior for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedBehavior {
    pub behavior_type: BehaviorType,
    pub description: String,
    pub validation: BehaviorValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorType {
    TriggerCondition,
    DataCollection,
    Decision,
    Action,
    Notification,
    ReportGeneration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorValidation {
    Boolean { should_be: bool },
    ValueRange { min: f64, max: f64 },
    ValueEquals { expected: f64 },
    StringContains { substring: String },
    CountEquals { expected: usize },
}

/// Builder for creating test scenarios
pub struct ScenarioBuilder {
    name: String,
    description: String,
    devices: Vec<SimulatedDevice>,
    time_patterns: Vec<TimePattern>,
    expected_behaviors: Vec<ExpectedBehavior>,
}

impl ScenarioBuilder {
    /// Create a new scenario builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            devices: Vec::new(),
            time_patterns: Vec::new(),
            expected_behaviors: Vec::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a device to the scenario
    pub fn add_device(mut self, device: SimulatedDevice) -> Self {
        self.devices.push(device);
        self
    }

    /// Add multiple devices
    pub fn add_devices(mut self, devices: Vec<SimulatedDevice>) -> Self {
        self.devices.extend(devices);
        self
    }

    /// Add a time pattern
    pub fn add_time_pattern(mut self, pattern: TimePattern) -> Self {
        self.time_patterns.push(pattern);
        self
    }

    /// Add an expected behavior
    pub fn add_expected_behavior(mut self, behavior: ExpectedBehavior) -> Self {
        self.expected_behaviors.push(behavior);
        self
    }

    /// Build the scenario
    pub fn build(self) -> Scenario {
        Scenario {
            name: self.name,
            description: self.description,
            devices: self.devices,
            time_patterns: self.time_patterns,
            expected_behaviors: self.expected_behaviors,
        }
    }

    /// Create a simulator from this scenario
    pub async fn create_simulator(self) -> (Scenario, DeviceSimulator) {
        let scenario = self.build();
        let simulator = DeviceSimulator::new();

        for device in scenario.devices.clone() {
            simulator.add_device(device).await;
        }

        (scenario, simulator)
    }
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new("Test Scenario")
    }
}

// ========== Predefined Scenario Builders ==========

/// Builder for basic monitoring scenarios
pub struct MonitoringScenarioBuilder {
    builder: ScenarioBuilder,
}

impl MonitoringScenarioBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            builder: ScenarioBuilder::new(name),
        }
    }

    /// Set up temperature monitoring
    pub fn temperature_monitoring(self, device_count: usize) -> Self {
        let mut devices = Vec::new();
        for i in 0..device_count {
            let device = SimulatedDevice::new(
                format!("temp-sensor-{}", i + 1),
                format!("温度传感器 {}", i + 1),
                SimulatedDeviceType::TemperatureSensor,
            )
            .with_location(format!("区域{}", i + 1))
            .with_base_value("temperature", 20.0 + i as f64 * 2.0);
            devices.push(device);
        }

        Self {
            builder: self.builder.add_devices(devices),
        }
    }

    /// Set up environment monitoring (temp + humidity)
    pub fn environment_monitoring(self, location_count: usize) -> Self {
        let mut devices = Vec::new();
        for i in 0..location_count {
            let temp_device = SimulatedDevice::new(
                format!("env-{}-temp", i + 1),
                format!("环境传感器 {} - 温度", i + 1),
                SimulatedDeviceType::TempHumiditySensor,
            )
            .with_location(format!("区域{}", i + 1))
            .with_base_value("temperature", 24.0);
            devices.push(temp_device);
        }

        Self {
            builder: self.builder.add_devices(devices),
        }
    }

    /// Add daily execution expectation
    pub fn with_daily_schedule(mut self, time: &str) -> Self {
        self.builder.time_patterns.push(TimePattern {
            pattern_type: TimePatternType::Daily,
            schedule: time.to_string(),
            devices: self
                .builder
                .devices
                .iter()
                .map(|d| d.id.clone())
                .collect(),
        });
        self
    }

    /// Build into the base builder
    pub fn build(self) -> ScenarioBuilder {
        self.builder
    }
}

/// Builder for report generation scenarios
pub struct ReportScenarioBuilder {
    builder: ScenarioBuilder,
}

impl ReportScenarioBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            builder: ScenarioBuilder::new(name)
                .with_description("Generate periodic reports from device data"),
        }
    }

    /// Set up daily report scenario
    pub fn daily_report(self) -> Self {
        Self {
            builder: self.builder.add_time_pattern(TimePattern {
                pattern_type: TimePatternType::Daily,
                schedule: "0 8 * * *".to_string(), // 8:00 AM daily
                devices: vec![], // Will be filled when devices are added
            })
        }
    }

    /// Set up weekly report scenario
    pub fn weekly_report(self) -> Self {
        Self {
            builder: self.builder.add_time_pattern(TimePattern {
                pattern_type: TimePatternType::Weekly,
                schedule: "0 9 * * 1".to_string(), // 9:00 AM Monday
                devices: vec![],
            })
        }
    }

    /// Add devices for the report
    pub fn with_devices(self, devices: Vec<SimulatedDevice>) -> Self {
        Self {
            builder: self.builder.add_devices(devices),
        }
    }

    /// Expect a report to be generated
    pub fn expect_report(self) -> Self {
        Self {
            builder: self.builder.add_expected_behavior(ExpectedBehavior {
                behavior_type: BehaviorType::ReportGeneration,
                description: "Generate a report with collected data".to_string(),
                validation: BehaviorValidation::Boolean { should_be: true },
            }),
        }
    }

    /// Build into the base builder
    pub fn build(self) -> ScenarioBuilder {
        self.builder
    }
}

/// Builder for anomaly detection scenarios
pub struct AnomalyScenarioBuilder {
    builder: ScenarioBuilder,
    generator: TestDataGenerator,
}

impl AnomalyScenarioBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            builder: ScenarioBuilder::new(name)
                .with_description("Detect and respond to anomalous data"),
            generator: TestDataGenerator::new(),
        }
    }

    /// Add a sensor with anomalies
    pub fn with_anomalies(
        self,
        sensor_id: impl Into<String>,
        normal_value: f64,
        _anomaly_value: f64,
        _anomaly_count: usize,
    ) -> Self {
        // Create a device with abnormal behavior capability
        let device = SimulatedDevice::new(
            sensor_id,
            "Anomaly Sensor",
            SimulatedDeviceType::TemperatureSensor,
        )
        .with_base_value("temperature", normal_value)
        .with_variance(1.0); // Low variance normally

        Self {
            builder: self.builder.add_device(device),
            generator: self.generator,
        }
    }

    /// Expect anomaly detection
    pub fn expect_detection(self) -> Self {
        Self {
            builder: self.builder.add_expected_behavior(ExpectedBehavior {
                behavior_type: BehaviorType::TriggerCondition,
                description: "Detect anomaly in data".to_string(),
                validation: BehaviorValidation::Boolean { should_be: true },
            }),
            generator: self.generator,
        }
    }

    /// Expect notification on anomaly
    pub fn expect_notification(self) -> Self {
        Self {
            builder: self.builder.add_expected_behavior(ExpectedBehavior {
                behavior_type: BehaviorType::Notification,
                description: "Send notification for anomaly".to_string(),
                validation: BehaviorValidation::Boolean { should_be: true },
            }),
            generator: self.generator,
        }
    }

    /// Build into the base builder
    pub fn build(self) -> ScenarioBuilder {
        self.builder
    }
}

// ========== Helper functions for common scenarios ==========

/// Create a simple temperature monitoring scenario
pub fn simple_temperature_scenario() -> Scenario {
    ScenarioBuilder::new("Simple Temperature Monitoring")
        .with_description("Monitor temperature and alert when it exceeds threshold")
        .add_devices(
            DeviceSimulatorBuilder::new()
                .add_temperature_sensors(3, "warehouse")
                .build(),
        )
        .add_time_pattern(TimePattern {
            pattern_type: TimePatternType::Interval,
            schedule: "*/5 * * * *".to_string(), // Every 5 minutes
            devices: vec!["warehouse-temp-1".to_string(), "warehouse-temp-2".to_string()],
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::DataCollection,
            description: "Collect temperature data every 5 minutes".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::TriggerCondition,
            description: "Trigger when temperature > 30°C".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .build()
}

/// Create a daily report scenario
pub fn daily_report_scenario() -> Scenario {
    ScenarioBuilder::new("Daily Report")
        .with_description("Generate daily report of warehouse conditions")
        .add_devices(
            DeviceSimulatorBuilder::new()
                .add_warehouse_environment(3)
                .build(),
        )
        .add_time_pattern(TimePattern {
            pattern_type: TimePatternType::Daily,
            schedule: "0 8 * * *".to_string(), // 8:00 AM
            devices: vec![],
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::DataCollection,
            description: "Collect 24 hours of data".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::ReportGeneration,
            description: "Generate summary report".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .build()
}

/// Create an anomaly detection scenario
pub fn anomaly_detection_scenario() -> Scenario {
    ScenarioBuilder::new("Anomaly Detection")
        .with_description("Detect anomalies in sensor data and alert")
        .add_devices(
            DeviceSimulatorBuilder::new()
                .add_temperature_sensors(5, "factory")
                .build(),
        )
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::TriggerCondition,
            description: "Detect values outside normal range".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::Notification,
            description: "Send alert when anomaly detected".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .build()
}

/// Create a complex multi-condition scenario
pub fn complex_monitoring_scenario() -> Scenario {
    ScenarioBuilder::new("Complex Multi-Condition Monitoring")
        .with_description("Monitor multiple conditions with cascading actions")
        .add_devices(
            DeviceSimulatorBuilder::new()
                .add_warehouse_environment(2)
                .add_smart_switches(4, "warehouse")
                .build(),
        )
        .add_time_pattern(TimePattern {
            pattern_type: TimePatternType::Interval,
            schedule: "*/2 * * * *".to_string(), // Every 2 minutes
            devices: vec![],
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::DataCollection,
            description: "Monitor temperature and humidity".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::Decision,
            description: "Evaluate complex conditions".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .add_expected_behavior(ExpectedBehavior {
            behavior_type: BehaviorType::Action,
            description: "Execute cascading actions".to_string(),
            validation: BehaviorValidation::Boolean { should_be: true },
        })
        .build()
}

/// Re-export DeviceSimulatorBuilder for convenience
pub use super::device_simulator::DeviceSimulatorBuilder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_builder() {
        let scenario = ScenarioBuilder::new("Test Scenario")
            .with_description("A test scenario")
            .add_device(SimulatedDevice::new(
                "test-1",
                "Test Device",
                SimulatedDeviceType::TemperatureSensor,
            ))
            .add_expected_behavior(ExpectedBehavior {
                behavior_type: BehaviorType::DataCollection,
                description: "Collect data".to_string(),
                validation: BehaviorValidation::Boolean { should_be: true },
            })
            .build();

        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.devices.len(), 1);
        assert_eq!(scenario.expected_behaviors.len(), 1);
    }

    #[test]
    fn test_predefined_scenarios() {
        let scenario = simple_temperature_scenario();
        assert_eq!(scenario.name, "Simple Temperature Monitoring");
        assert!(!scenario.devices.is_empty());

        let scenario = daily_report_scenario();
        assert_eq!(scenario.name, "Daily Report");

        let scenario = anomaly_detection_scenario();
        assert_eq!(scenario.name, "Anomaly Detection");

        let scenario = complex_monitoring_scenario();
        assert_eq!(scenario.name, "Complex Multi-Condition Monitoring");
    }
}
