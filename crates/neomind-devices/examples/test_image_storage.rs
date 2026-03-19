// Test image storage and retrieval
use base64::{engine::general_purpose::STANDARD, Engine};
use neomind_devices::mdl::MetricValue;
use neomind_devices::telemetry::{DataPoint, TimeSeriesStorage};

fn get_project_data_path(filename: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join(filename)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_project_data_path("test_images.redb");
    let storage = TimeSeriesStorage::open(&db_path)?;

    let now = chrono::Utc::now().timestamp();

    // Create a small PNG (1x1 red pixel)
    // This is a valid PNG file in base64
    let png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let png_bytes = STANDARD.decode(png_base64)?;

    // Store as binary metric
    storage
        .write(
            "camera_device",
            "image",
            DataPoint::new(now, MetricValue::Binary(png_bytes)),
        )
        .await?;

    println!("Image data stored successfully!");

    // Query it back
    let results = storage
        .query("camera_device", "image", now - 10, now + 10)
        .await?;
    println!("Retrieved {} data points", results.len());

    for point in results {
        match &point.value {
            MetricValue::Binary(data) => {
                let encoded = STANDARD.encode(data);
                println!("Binary data (base64): {}...", &encoded[..50]);

                // Check if it starts with PNG signature
                if encoded.starts_with("iVBORw0KGgo") {
                    println!("This is a PNG image!");
                }
            }
            _ => println!("Not binary data: {:?}", point.value),
        }
    }

    Ok(())
}
