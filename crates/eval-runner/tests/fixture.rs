use eval_runner::fixture::load_fixture;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // tests run with CWD = crate dir; workspace root is 2 levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

#[test]
fn load_seed_empty() {
    let path = workspace_root().join("eval/fixtures/seed-empty.json");
    let fix = load_fixture(&path).unwrap();
    assert_eq!(fix.name, "seed-empty");
    assert!(fix.devices.is_empty());
}

#[test]
fn load_seed_default_has_devices() {
    let path = workspace_root().join("eval/fixtures/seed-default.json");
    let fix = load_fixture(&path).unwrap();
    assert_eq!(fix.name, "seed-default");
    assert!(!fix.devices.is_empty());
    assert!(!fix.metrics.is_empty());
}
