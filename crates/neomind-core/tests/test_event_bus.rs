//! Simple standalone test for EventBus functionality
use neomind_core::{EventBus, event::NeoMindEvent, MetricValue};

fn main() {
    println!("=== EventBus ExtensionOutput Event Test ===\n");

    // Create event bus
    let event_bus = EventBus::new();
    let mut receiver = event_bus.receiver().clone();

    // Start listening for events
    let event_bus_clone = event_bus.clone();
    let handle = std::thread::spawn(move || {
        let mut count = 0;
        while let Ok(event) = receiver.recv() {
            count += 1;
            println!("[{}] Event #{}: {}",
                std::any::type_name::<NeoMindEvent>(),
                count);
        }
        println!("Total events received: {}", count);
    });

    // Give listener time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Test: Publish an ExtensionOutput event manually
    println!("\n--- Publishing ExtensionOutput event ---");
    let test_event = NeoMindEvent::ExtensionOutput {
        extension_id: "test_ext",
        output_name: "temperature",
        value: MetricValue::Float(23.5),
        timestamp: chrono::Utc::now().timestamp(),
        labels: None,
        quality: None,
    };

    match event_bus.publish(test_event) {
        Ok(_) => println!("Published successfully"),
        Err(e) => println!("Publish failed: {:?}", e),
    }

    // Give time for event to be received
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Clean up
    drop(receiver);
    drop(event_bus_clone);
    handle.join().unwrap();

    println!("\n=== Test Complete ===");
}
