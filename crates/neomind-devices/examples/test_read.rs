use neomind_devices::telemetry::TimeSeriesStorage;

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

    // Try to read all metric types
    println!("Testing data persistence after restart...");

    let metrics = vec!["metric.int", "metric.float", "metric.string", "metric.bool"];

    for metric in metrics {
        let results = storage
            .query("device1", metric, now - 1000, now + 1000)
            .await?;
        println!("{}: {} data points", metric, results.len());
        for point in results {
            println!("  Value: {:?}", point.value);
        }
    }

    Ok(())
}
