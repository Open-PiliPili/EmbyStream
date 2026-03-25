use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use tokio::time::interval;

const LOG_RETENTION_DAYS: u64 = 7;
const CLEANUP_INTERVAL_HOURS: u64 = 24;

/// Starts a background task to clean up old log files.
/// Removes log files older than LOG_RETENTION_DAYS (7 days).
pub fn start_cleanup_task(log_directory: String) {
    tokio::spawn(async move {
        let mut ticker =
            interval(Duration::from_secs(CLEANUP_INTERVAL_HOURS * 60 * 60));

        loop {
            ticker.tick().await;
            if let Err(e) = cleanup_old_logs(&log_directory) {
                eprintln!("Log cleanup error: {}", e);
            }
        }
    });
}

fn cleanup_old_logs(log_directory: &str) -> Result<(), String> {
    let log_path = Path::new(log_directory);
    if !log_path.exists() {
        return Ok(());
    }

    let cutoff_time = SystemTime::now()
        .checked_sub(Duration::from_secs(LOG_RETENTION_DAYS * 24 * 60 * 60))
        .ok_or("Failed to calculate cutoff time")?;

    let entries = fs::read_dir(log_path)
        .map_err(|e| format!("Failed to read log directory: {}", e))?;

    let mut removed_count = 0;
    let mut total_size: u64 = 0;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !is_log_file(&path) {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = match metadata.modified() {
            Ok(t) => t,
            Err(_) => continue,
        };

        if modified < cutoff_time {
            let file_size = metadata.len();
            if fs::remove_file(&path).is_ok() {
                removed_count += 1;
                total_size += file_size;
            }
        }
    }

    if removed_count > 0 {
        println!(
            "Log cleanup: removed {} old log files ({} bytes)",
            removed_count, total_size
        );
    }

    Ok(())
}

fn is_log_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    if let Some(ext) = path.extension() {
        if ext == "log" {
            return true;
        }
    }

    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        return name_str.contains(".log");
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn is_log_file_detects_log_extension() {
        use std::fs::File;
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.log");
        File::create(&path).unwrap();
        assert!(is_log_file(&path));
    }

    #[test]
    fn is_log_file_detects_log_in_name() {
        use std::fs::File;
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("app.log.2024-01-01");
        File::create(&path).unwrap();
        assert!(is_log_file(&path));
    }

    #[test]
    fn is_log_file_rejects_non_log() {
        use std::fs::File;
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        File::create(&path).unwrap();
        assert!(!is_log_file(&path));
    }

    #[test]
    fn cleanup_removes_old_files() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let log_dir = temp_dir.path();

        let old_file = log_dir.join("old.log");
        let mut file = File::create(&old_file)?;
        file.write_all(b"old log content")?;
        drop(file);

        let old_time = SystemTime::now()
            .checked_sub(Duration::from_secs(8 * 24 * 60 * 60))
            .unwrap();
        filetime::set_file_mtime(
            &old_file,
            filetime::FileTime::from(old_time),
        )?;

        let new_file = log_dir.join("new.log");
        let mut file = File::create(&new_file)?;
        file.write_all(b"new log content")?;
        drop(file);

        cleanup_old_logs(log_dir.to_str().unwrap())?;

        assert!(!old_file.exists(), "Old file should be removed");
        assert!(new_file.exists(), "New file should remain");

        Ok(())
    }

    #[test]
    fn cleanup_handles_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = cleanup_old_logs(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn cleanup_handles_nonexistent_directory() {
        let result = cleanup_old_logs("/nonexistent/path/to/logs");
        assert!(result.is_ok());
    }
}
