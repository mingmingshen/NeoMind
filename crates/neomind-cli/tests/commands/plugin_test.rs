//! Tests for the `plugin` command and its subcommands.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs::{self, File};
use std::io::Write;

/// Test that plugin command requires a subcommand.
#[test]
fn test_plugin_requires_subcommand() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin");

    // Clap displays usage on stdout with exit code 2
    cmd.assert()
        .failure()
        .code(2);
}

/// Test plugin validate requires path argument.
#[test]
fn test_plugin_validate_requires_path() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("validate");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stderr(predicate::str::contains("<PATH>"));
}

/// Test plugin validate with verbose flag.
#[test]
fn test_plugin_validate_verbose_flag() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("validate")
        .arg("/nonexistent/path.wasm")
        .arg("--verbose");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}

/// Test plugin create command.
#[test]
fn test_plugin_create_command() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("create")
        .arg("test-plugin")
        .arg("--plugin-type")
        .arg("tool");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Creating plugin"))
        .stdout(predicate::str::contains("test-plugin"))
        .stdout(predicate::str::contains("tool"));
}

/// Test plugin create with invalid type shows valid types.
#[test]
fn test_plugin_create_invalid_type() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("create")
        .arg("test-plugin")
        .arg("--plugin-type")
        .arg("invalid-type");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid plugin type"))
        .stderr(predicate::str::contains("Valid types:"));
}

/// Test plugin create help shows all valid types.
#[test]
fn test_plugin_create_help() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("create")
        .arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Create a new plugin scaffold"))
        .stdout(predicate::str::contains("--plugin-type"));
}

/// Test plugin list command works.
#[test]
fn test_plugin_list_command() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Discovered Plugins"));
}

/// Test plugin list with custom directory.
#[test]
fn test_plugin_list_with_dir() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("list")
        .arg("--dir")
        .arg(temp_dir.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Discovered Plugins"));
}

/// Test plugin list with type filter.
#[test]
fn test_plugin_list_with_type() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("list")
        .arg("--ty")
        .arg("device_adapter");

    cmd.assert()
        .success();
}

/// Test plugin info requires path.
#[test]
fn test_plugin_info_requires_path() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("info");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stderr(predicate::str::contains("<PATH>"));
}

/// Test plugin info with nonexistent file.
#[test]
fn test_plugin_info_nonexistent_file() {
    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("info")
        .arg("/tmp/nonexistent-extension-12345.wasm");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test plugin validate with a fake WASM file.
#[test]
fn test_plugin_validate_fake_wasm() {
    let temp_dir = TempDir::new().unwrap();
    let fake_wasm = temp_dir.path().join("test.wasm");

    let mut file = File::create(&fake_wasm).unwrap();
    file.write_all(b"fake wasm content").unwrap();

    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("validate")
        .arg(&fake_wasm);

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}

/// Test plugin validate with a fake native library.
#[test]
fn test_plugin_validate_fake_native() {
    let temp_dir = TempDir::new().unwrap();
    let extension = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };
    let fake_lib = temp_dir.path().join(&format!("test.{}", extension));

    let mut file = File::create(&fake_lib).unwrap();
    file.write_all(b"fake library content").unwrap();

    let mut cmd = Command::cargo_bin("neomind").unwrap();
    cmd.arg("plugin")
        .arg("validate")
        .arg(&fake_lib);

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("FAILED"));
}
