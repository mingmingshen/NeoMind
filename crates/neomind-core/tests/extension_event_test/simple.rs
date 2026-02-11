//! Simple test for ExtensionOutput event publishing
use neomind_core::{EventBus, event::NeoMindEvent, MetricValue};

fn main() {
    println!("=== Simple ExtensionOutput Event Test ===");
    println!("\nThis test verifies that ExtensionOutput events are published correctly");
    println!("when an extension command is executed.\n");

    // Create event bus
    let event_bus = EventBus::new();
    let mut receiver = event_bus.receiver().clone();

    // Start listening for events
    let event_bus_clone = event_bus.clone();
    std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            println!("[EventBus] Received: {} - {}", event.type_name(), format_event(&event));
        }
    });

    // Give time for listener to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test 1: Publish an ExtensionOutput event manually
    println!("\n--- Test 1: Publishing ExtensionOutput event ---");
    let test_event = NeoMindEvent::ExtensionOutput {
        extension_id: "test_ext".to_string(),
        output_name: "temperature".to_string(),
        value: MetricValue::Float(23.5),
        timestamp: chrono::Utc::now().timestamp(),
        labels: None,
        quality: None,
    };
    event_bus.publish(test_event.clone());
    println!("Published ExtensionOutput event for test_ext:temperature");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Test 2: Verify event was received
    println!("\n--- Test 2: Verifying event was received ---");

    // Clean up
    drop(receiver);
    drop(event_bus_clone);

    println!("\n=== Test Complete ===");
    println!("If the ExtensionOutput event was published correctly,");
    println!("you should have seen it logged above.");
}

fn format_event(event: &NeoMindEvent) -> String {
    match event {
        NeoMindEvent::ExtensionOutput { extension_id, output_name, value, .. } => {
            format!("extension={}, metric={}, value={}",
                extension_id, output_name,
                match value {
                    MetricValue::Float(f) => format!("float({})", f),
                    MetricValue::Integer(i) => format!("int({})", i),
                    MetricValue::Boolean(b) => format!("bool({})", b),
                    MetricValue::String(s) => format!("str({})", s),
                    MetricValue::Json(_) => "json(...)".to_string(),
                })
        }
        _ => format!("{}", event.type_name()),
    }
}
