//! Discover valid EmbyStream config TOML files in a directory.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::config::core::{
    parse_raw_config_str, validate_raw_regexes, validate_raw_structure,
};

/// Entry for a discovered config file.
#[derive(Debug, Clone)]
pub struct DiscoveredConfig {
    pub path: PathBuf,
    pub stream_mode: String,
}

/// List `*.toml` in `dir` that parse as `RawConfig` and pass structure + regex checks.
pub fn discover_configs(dir: &Path) -> io::Result<Vec<DiscoveredConfig>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("toml"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let raw = match parse_raw_config_str(&content) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if validate_raw_structure(&raw).is_err() {
            continue;
        }
        if validate_raw_regexes(&raw).is_err() {
            continue;
        }
        let mode = raw.general.stream_mode.to_string();
        out.push(DiscoveredConfig {
            path,
            stream_mode: mode,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use tempfile::tempdir;

    #[test]
    fn sorts_and_skips_invalid() {
        let dir = tempdir().expect("tempdir");
        let mut f1 =
            std::fs::File::create(dir.path().join("b.toml")).expect("create");
        writeln!(f1, "not toml").expect("write");
        let list = discover_configs(dir.path()).expect("discover");
        assert!(list.is_empty());
    }
}
