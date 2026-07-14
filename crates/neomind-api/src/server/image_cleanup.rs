//! Image file retention cleanup.
//!
//! Provides cleanup functionality for expired image files stored in
//! `data/images/<device>/<metric>/<timestamp>.<ext>`. This prevents
//! disk space leaks from accumulating image data over time.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cleanup expired image files from the images directory.
///
/// This function recursively scans the `images_dir` and deletes files
/// whose timestamp (extracted from the filename) is older than the
/// specified retention period. It also cleans up empty directories.
///
/// # Arguments
///
/// * `images_dir` - Path to the images directory (e.g., `data/images`)
/// * `retention_hours` - Retention period in hours (files older than this are deleted)
///
/// # File Format
///
/// Images must be stored as: `<device>/<metric>/<timestamp>.<ext>`
/// where `timestamp` is milliseconds since Unix epoch.
///
/// # Returns
///
/// Returns the number of files deleted and the number of directories cleaned up.
pub async fn cleanup_expired_images(
    images_dir: &Path,
    retention_hours: u64,
) -> anyhow::Result<(usize, usize)> {
    let retention_duration = Duration::from_secs(retention_hours * 3600);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Failed to get current time: {}", e))?;

    let cutoff_timestamp_ms = now
        .checked_sub(retention_duration)
        .ok_or_else(|| anyhow::anyhow!("Invalid retention duration: would underflow"))?
        .as_millis() as i64;

    tracing::debug!(
 retention_hours = retention_hours,
        cutoff_timestamp_ms = cutoff_timestamp_ms,
        "Starting image cleanup"
    );

    if !images_dir.exists() {
        tracing::debug!("Images directory does not exist, skipping cleanup");
        return Ok((0, 0));
    }

    let mut files_deleted = 0usize;
    let dirs_cleaned;

    // Collect all files first to avoid borrowing issues
    let files_to_delete: Vec<(PathBuf, i64)> = collect_expired_files(images_dir, cutoff_timestamp_ms)?;

    // Delete files
    for (file_path, timestamp_ms) in &files_to_delete {
        match fs::remove_file(file_path) {
            Ok(_) => {
                files_deleted += 1;
                tracing::debug!(
                    file = %file_path.display(),
                    timestamp_ms = timestamp_ms,
                    "Deleted expired image file"
                );
            }
            Err(e) => {
                tracing::warn!(
                    file = %file_path.display(),
                    error = %e,
                    "Failed to delete expired image file"
                );
            }
        }
    }

    // Clean up empty directories
    dirs_cleaned = cleanup_empty_directories(images_dir)?;

    if files_deleted > 0 || dirs_cleaned > 0 {
        tracing::info!(
            files_deleted = files_deleted,
            dirs_cleaned = dirs_cleaned,
            retention_hours = retention_hours,
            "Image cleanup completed"
        );
    } else {
        tracing::debug!("No expired images found to clean up");
    }

    Ok((files_deleted, dirs_cleaned))
}

/// Collect all expired image files from the images directory.
///
/// This function recursively scans the directory and collects files
/// whose timestamp is older than the cutoff.
fn collect_expired_files(
    images_dir: &Path,
    cutoff_timestamp_ms: i64,
) -> anyhow::Result<Vec<(PathBuf, i64)>> {
    let mut expired_files = Vec::new();

    let entries = match fs::read_dir(images_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!(
                dir = %images_dir.display(),
                error = %e,
                "Failed to read images directory"
            );
            return Ok(expired_files);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            // Recursively process subdirectories
            let sub_files = collect_expired_files(&path, cutoff_timestamp_ms)?;
            expired_files.extend(sub_files);
        } else if file_type.is_file() {
            // Check if this is an image file with timestamp
            if let Some(timestamp_ms) = extract_timestamp_from_filename(&path) {
                if timestamp_ms < cutoff_timestamp_ms {
                    expired_files.push((path, timestamp_ms));
                }
            }
        }
    }

    Ok(expired_files)
}

/// Extract timestamp from filename.
///
/// Expects filename format: `<timestamp>.<ext>` where timestamp is
/// milliseconds since Unix epoch.
fn extract_timestamp_from_filename(path: &Path) -> Option<i64> {
    let filename = path.file_name()?.to_str()?;

    // Split by extension
    let parts: Vec<&str> = filename.rsplitn(2, '.').collect();
    if parts.len() != 2 || parts.is_empty() {
        return None;
    }

    // The timestamp is the part before the extension
    let timestamp_str = parts.get(1)?;

    // Parse as i64 (milliseconds)
    timestamp_str.parse::<i64>().ok()
}

/// Clean up empty directories in the images directory.
///
/// Returns the number of empty directories removed.
fn cleanup_empty_directories(images_dir: &Path) -> anyhow::Result<usize> {
    let mut dirs_cleaned = 0usize;

    // Collect all directories first (bottom-up)
    let mut all_dirs = Vec::new();

    fn collect_all_dirs(dir: &Path, all_dirs: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Ok(()),
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_dir() {
                all_dirs.push(path.clone());
                collect_all_dirs(&path, all_dirs)?;
            }
        }

        Ok(())
    }

    collect_all_dirs(images_dir, &mut all_dirs)?;

    // Sort by depth (deepest first)
    all_dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

    // Remove empty directories
    for dir in all_dirs {
        // Check if directory is empty
        let is_empty = match fs::read_dir(&dir) {
            Ok(mut entries) => entries.next().is_none(),
            Err(_) => false,
        };

        if is_empty {
            match fs::remove_dir(&dir) {
                Ok(_) => {
                    dirs_cleaned += 1;
                    tracing::debug!(dir = %dir.display(), "Removed empty directory");
                }
                Err(e) => {
                    tracing::warn!(
                        dir = %dir.display(),
                        error = %e,
                        "Failed to remove empty directory"
                    );
                }
            }
        }
    }

    Ok(dirs_cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn create_test_image_file(dir: &Path, filename: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test image data").unwrap();
        file_path
    }

    fn create_test_dir_structure(base_dir: &Path) -> PathBuf {
        let device_dir = base_dir.join("device-001");
        let metric_dir = device_dir.join("image");
        fs::create_dir_all(&metric_dir).unwrap();
        metric_dir
    }

    #[test]
    fn test_extract_timestamp_from_filename() {
        // Valid timestamps
        assert_eq!(
            extract_timestamp_from_filename(Path::new("1634567890000.jpg")),
            Some(1634567890000)
        );
        assert_eq!(
            extract_timestamp_from_filename(Path::new("123456789.png")),
            Some(123456789)
        );
        assert_eq!(
            extract_timestamp_from_filename(Path::new("0.webp")),
            Some(0)
        );

        // Invalid filenames
        assert_eq!(extract_timestamp_from_filename(Path::new("notimestamp.jpg")), None);
        assert_eq!(extract_timestamp_from_filename(Path::new("text.txt")), None);
        assert_eq!(extract_timestamp_from_filename(Path::new("163abc.jpg")), None);
    }

    #[tokio::test]
    async fn test_cleanup_expired_images() {
        // Create temporary directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        let images_dir = temp_dir.path().join("images");
        let metric_dir = create_test_dir_structure(&images_dir);

        // Create test files with different timestamps
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Recent file (within retention period) - 1 hour ago
        let recent_file = create_test_image_file(&metric_dir, &format!("{}.jpg", now - 3600_000));

        // Expired file (older than retention period) - 5 hours ago
        let expired_file = create_test_image_file(&metric_dir, &format!("{}.jpg", now - 18000_000));

        // Run cleanup with 2 hours retention
        let result = cleanup_expired_images(&images_dir, 2).await.unwrap();

        // Assert: recent file should exist, expired file should be deleted
        assert!(recent_file.exists());
        assert!(!expired_file.exists());
        assert_eq!(result, (1, 0)); // 1 file deleted, 0 dirs cleaned
    }

    #[tokio::test]
    async fn test_cleanup_with_empty_directories() {
        // Create temporary directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        let images_dir = temp_dir.path().join("images");

        // Create nested directory structure with only expired files
        let device_dir = images_dir.join("device-002");
        let metric_dir = device_dir.join("temperature");
        fs::create_dir_all(&metric_dir).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let expired_file = create_test_image_file(&metric_dir, &format!("{}.jpg", now - 5_000_000)); // ~1.4 hours ago

        // Run cleanup with 1 hour retention to ensure file is deleted
        let result = cleanup_expired_images(&images_dir, 1).await.unwrap();

        // Assert: file should be deleted and empty directories should be cleaned
        assert!(!expired_file.exists());
        assert!(!metric_dir.exists()); // empty metric dir should be removed
        assert!(!device_dir.exists()); // empty device dir should be removed
        assert_eq!(result.0, 1); // 1 file deleted
        assert!(result.1 > 0); // some dirs cleaned
    }

    #[tokio::test]
    async fn test_cleanup_nonexistent_directory() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let nonexistent_dir = temp_dir.path().join("nonexistent");

        let result = cleanup_expired_images(&nonexistent_dir, 72).await.unwrap();
        assert_eq!(result, (0, 0));
    }

    #[tokio::test]
    async fn test_cleanup_no_expired_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let images_dir = temp_dir.path().join("images");
        let metric_dir = create_test_dir_structure(&images_dir);

        // Create only recent files
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let recent_file = create_test_image_file(&metric_dir, &format!("{}.jpg", now));

        // Run cleanup with long retention
        let result = cleanup_expired_images(&images_dir, 720).await.unwrap(); // 30 days

        // Assert: no files should be deleted
        assert!(recent_file.exists());
        assert_eq!(result, (0, 0));
    }

    #[tokio::test]
    async fn test_cleanup_multiple_devices() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let images_dir = temp_dir.path().join("images");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Create multiple devices with mixed file ages
        for device_id in 1..=3 {
            let device_dir = images_dir.join(format!("device-{:03}", device_id));
            let metric_dir = device_dir.join("image");
            fs::create_dir_all(&metric_dir).unwrap();

            // Recent file (1 hour ago)
            create_test_image_file(&metric_dir, &format!("{}.jpg", now - 3_600_000));

            // Expired file (5 hours ago)
            create_test_image_file(&metric_dir, &format!("{}.jpg", now - 18_000_000));
        }

        // Run cleanup with 2 hours retention
        let result = cleanup_expired_images(&images_dir, 2).await.unwrap();

        // Assert: 3 expired files deleted, 3 recent files remain
        assert_eq!(result.0, 3); // 3 files deleted
    }
}