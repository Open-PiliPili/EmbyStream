use crate::config::backend::GoogleDriveConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveLookup {
    DriveId(String),
    DriveName(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedGoogleDrivePath {
    pub lookup: DriveLookup,
    pub drive_name: String,
    pub logical_path: String,
    pub relative_path: String,
}

fn normalize_path_segments(path: &str) -> Vec<&str> {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn join_segments(segments: &[&str]) -> String {
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

pub fn resolve_google_drive_path(
    raw_path: &str,
    cfg: &GoogleDriveConfig,
) -> Result<ResolvedGoogleDrivePath, &'static str> {
    let segments = normalize_path_segments(raw_path);
    if segments.is_empty() {
        return Err("googleDrive path is empty");
    }

    let configured_drive_name = cfg.drive_name.trim();
    let drive_name_in_path_index = if configured_drive_name.is_empty() {
        None
    } else {
        segments
            .iter()
            .position(|segment| *segment == configured_drive_name)
    };

    let logical_segments: Vec<&str> = match drive_name_in_path_index {
        Some(idx) => segments[idx..].to_vec(),
        None if !configured_drive_name.is_empty() => {
            let mut merged = Vec::with_capacity(segments.len() + 1);
            merged.push(configured_drive_name);
            merged.extend(segments.iter().copied());
            merged
        }
        None => segments.clone(),
    };

    let drive_name = if configured_drive_name.is_empty() {
        logical_segments[0].to_string()
    } else {
        configured_drive_name.to_string()
    };

    let relative_segments = if logical_segments
        .first()
        .copied()
        .is_some_and(|segment| segment == drive_name)
    {
        &logical_segments[1..]
    } else {
        &logical_segments
    };

    let lookup = if cfg.drive_id.trim().is_empty() {
        DriveLookup::DriveName(drive_name.clone())
    } else {
        DriveLookup::DriveId(cfg.drive_id.trim().to_string())
    };

    Ok(ResolvedGoogleDrivePath {
        lookup,
        drive_name,
        logical_path: join_segments(&logical_segments),
        relative_path: join_segments(relative_segments),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_drive_name_from_first_segment_when_config_is_empty() {
        let cfg = GoogleDriveConfig::default();
        let resolved = resolve_google_drive_path(
            "/pilipili/pilipili/电视剧/2026/test/Season 01/test.mkv",
            &cfg,
        )
        .expect("resolve");

        assert_eq!(resolved.lookup, DriveLookup::DriveName("pilipili".into()));
        assert_eq!(resolved.drive_name, "pilipili");
        assert_eq!(
            resolved.logical_path,
            "/pilipili/pilipili/电视剧/2026/test/Season 01/test.mkv"
        );
        assert_eq!(
            resolved.relative_path,
            "/pilipili/电视剧/2026/test/Season 01/test.mkv"
        );
    }

    #[test]
    fn configured_drive_name_crops_prefix_and_keeps_duplicate_folder() {
        let cfg = GoogleDriveConfig {
            drive_name: "pilipili".into(),
            ..Default::default()
        };
        let resolved = resolve_google_drive_path(
            "/mnt/media/pilipili/pilipili/电视剧/2026/test/Season 01/test.mkv",
            &cfg,
        )
        .expect("resolve");

        assert_eq!(resolved.lookup, DriveLookup::DriveName("pilipili".into()));
        assert_eq!(
            resolved.logical_path,
            "/pilipili/pilipili/电视剧/2026/test/Season 01/test.mkv"
        );
        assert_eq!(
            resolved.relative_path,
            "/pilipili/电视剧/2026/test/Season 01/test.mkv"
        );
    }

    #[test]
    fn configured_drive_name_is_prepended_when_path_lacks_drive_segment() {
        let cfg = GoogleDriveConfig {
            drive_name: "pilipili".into(),
            ..Default::default()
        };
        let resolved =
            resolve_google_drive_path("/电影/2026/test/test.mkv", &cfg)
                .expect("resolve");

        assert_eq!(resolved.logical_path, "/pilipili/电影/2026/test/test.mkv");
        assert_eq!(resolved.relative_path, "/电影/2026/test/test.mkv");
    }

    #[test]
    fn drive_id_takes_precedence_over_drive_name_for_lookup() {
        let cfg = GoogleDriveConfig {
            drive_id: "drive-123".into(),
            drive_name: "pilipili".into(),
            ..Default::default()
        };
        let resolved = resolve_google_drive_path(
            "/mnt/media/pilipili/电影/2026/test/test.mkv",
            &cfg,
        )
        .expect("resolve");

        assert_eq!(resolved.lookup, DriveLookup::DriveId("drive-123".into()));
        assert_eq!(resolved.drive_name, "pilipili");
        assert_eq!(resolved.logical_path, "/pilipili/电影/2026/test/test.mkv");
        assert_eq!(resolved.relative_path, "/电影/2026/test/test.mkv");
    }
}
