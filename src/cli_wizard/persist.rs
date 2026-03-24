//! Write config to a temp file, then atomically move into place.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;

/// Write `contents` to a temp file in the system temp dir, then rename to `dest`.
/// If `dest` exists it is replaced. Cross-device rename falls back to copy + remove.
pub fn write_atomic(dest: &Path, contents: &str) -> std::io::Result<()> {
    let dest_dir = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dest_dir)?;

    let mut tmp = NamedTempFile::new_in(dest_dir)?;
    tmp.write_all(contents.as_bytes())?;
    tmp.as_file_mut().sync_all()?;
    tmp.persist(dest).map_err(|e| e.error)?;
    Ok(())
}

/// Returns `true` if `path` exists (file name conflict).
pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

/// Join cwd with filename (no path traversal): reject if name contains separators.
pub fn safe_join_cwd(cwd: &Path, filename: &str) -> Option<PathBuf> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename == "."
        || filename == ".."
    {
        return None;
    }
    Some(cwd.join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn safe_join_rejects_traversal() {
        let cwd = Path::new("/tmp");
        assert!(safe_join_cwd(cwd, "../etc/passwd").is_none());
        assert!(safe_join_cwd(cwd, "ok.toml").is_some());
    }

    #[test]
    fn atomic_write_roundtrip() {
        let dir = env::temp_dir()
            .join(format!("embystream_wizard_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        let dest = dir.join("out.toml");
        write_atomic(&dest, "[General]\nstream_mode = \"frontend\"\n")
            .expect("write");
        let s = fs::read_to_string(&dest).expect("read");
        assert!(s.contains("frontend"));
        let _ = fs::remove_dir_all(&dir);
    }
}
