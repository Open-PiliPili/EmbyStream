use std::path::{Path, PathBuf};

/// Resolve fallback video path (absolute or relative to config directory)
///
/// # Arguments
///
/// * `path_str` - The path string to resolve (can be absolute or relative)
/// * `config_path` - The path to the config file (used as base for relative paths)
///
/// # Returns
///
/// Returns `Some(String)` if the resolved path exists, `None` otherwise.
pub fn resolve_fallback_video_path(
    path_str: &str,
    config_path: &Path,
) -> Option<String> {
    if path_str.is_empty() {
        return None;
    }

    let path = PathBuf::from(path_str);
    let resolved_path = if path.is_absolute() {
        path
    } else {
        config_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path)
    };

    if resolved_path.exists() {
        Some(resolved_path.to_string_lossy().into_owned())
    } else {
        None
    }
}
