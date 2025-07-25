use std::{fs, path::Path};

use tokio::process::Command;

use super::types::FfprobeOutput;
use crate::{
    HLS_STREAM_LOGGER_DOMAIN, cache::transcoding::HlsConfig, debug_log,
    error_log, info_log,
};

pub async fn generate_m3u8_playlist(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<(), String> {
    debug_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Starting m3u8 playlist for input path: {:?}",
        input_path
    );
    let input_str = input_path.to_str().ok_or("Invalid input path")?;

    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .arg(input_str)
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let ffprobe_data: FfprobeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse ffprobe JSON output: {}", e))?;
    debug_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "ffprobe successfully parsed: {:?} for input path: {:?}",
        ffprobe_data,
        input_path
    );

    let duration_secs = ffprobe_data.format.duration;
    let segment_duration = config.segment_duration_seconds as f64;

    if duration_secs <= 0.0 || segment_duration <= 0.0 {
        return Err("Invalid video duration or segment duration".to_string());
    }
    let num_segments = (duration_secs / segment_duration).ceil() as usize;

    let mut playlist_content = String::new();
    playlist_content.push_str("#EXTM3U\n");
    playlist_content.push_str("#EXT-X-VERSION:3\n");
    playlist_content.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n");
    playlist_content.push_str(&format!(
        "#EXT-X-TARGETDURATION:{}\n",
        segment_duration.ceil() as u32
    ));
    playlist_content.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

    let mut remaining_secs = duration_secs;
    for i in 0..num_segments {
        let current_segment_duration = if remaining_secs > segment_duration {
            segment_duration
        } else {
            remaining_secs
        };
        if current_segment_duration > 0.01 {
            playlist_content.push_str(&format!(
                "#EXTINF:{:.6},\n",
                current_segment_duration
            ));
            playlist_content.push_str(&format!("segment{:05}.ts\n", i));
            remaining_secs -= current_segment_duration;
        }
    }

    playlist_content.push_str("#EXT-X-ENDLIST\n");

    let manifest_path = output_dir.join("master.m3u8");
    debug_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Writing final M3U8 playlist to: {:?}",
        manifest_path
    );

    if let Err(e) = fs::write(&manifest_path, playlist_content) {
        error_log!(
            HLS_STREAM_LOGGER_DOMAIN,
            "Failed to write M3U8 file to disk: {}",
            e
        );
        return Err(e.to_string());
    }

    info_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Instantly generated M3U8 playlist via ffprobe at: {:?}",
        manifest_path
    );

    Ok(())
}
