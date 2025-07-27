use std::path::Path;

use tokio::process::{Child, Command};

use crate::{
    HLS_STREAM_LOGGER_DOMAIN, cache::transcoding::HlsConfig, info_log,
};

pub async fn transmux_to_hls_segments(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<Child, String> {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-i")
        .arg(input_path.to_str().unwrap())
        .arg("-map")
        .arg("0:v:0?")
        .arg("-map")
        .arg("0:a?")
        .arg("-map")
        .arg("0:s?")
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("copy")
        .arg("-c:s")
        .arg("webvtt")
        .arg("-f")
        .arg("hls")
        .arg("-hls_time")
        .arg(config.segment_duration_seconds.to_string())
        .arg("-hls_playlist_type")
        .arg("vod")
        .arg("-hls_flags")
        .arg("single_file")
        .arg("-hls_segment_filename")
        .arg(output_dir.join("video.ts").to_str().unwrap())
        .arg(output_dir.join("video.m3u8"));

    let command_string = format!(
        "Executing FFmpeg Segments: {} {}",
        command.as_std().get_program().to_string_lossy(),
        command
            .as_std()
            .get_args()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    info_log!(HLS_STREAM_LOGGER_DOMAIN, "{}", command_string);

    let child = command.spawn().map_err(|e| format!("Failed to execute ffmpeg. Is it installed and in your PATH? Error: {}", e))?;

    Ok(child)
}
