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
    let db_path = get_project_data_path("telemetry_persistence_test.redb");
    let storage = TimeSeriesStorage::open(&db_path)?;

    let now = chrono::Utc::now().timestamp();

    // Write all types of data
    storage
        .write(
            "device1",
            "metric.int",
            DataPoint::new(now, MetricValue::Integer(100)),
        )
        .await?;
    storage
        .write(
            "device1",
            "metric.float",
            DataPoint::new(now, MetricValue::Float(99.5)),
        )
        .await?;
    storage
        .write(
            "device1",
            "metric.string",
            DataPoint::new(now, MetricValue::String("hello".to_string())),
        )
        .await?;
    storage
        .write(
            "device1",
            "metric.bool",
            DataPoint::new(now, MetricValue::Boolean(true)),
        )
        .await?;

    println!("Data written successfully!");

    // Query back to verify
    let results = storage
        .query("device1", "metric.string", now - 10, now + 10)
        .await?;
    println!("Queried {} data points for metric.string", results.len());
    for point in results {
        println!("  Value: {:?}", point.value);
    }

    Ok(())
}
