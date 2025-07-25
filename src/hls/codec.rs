use std::{path::Path, process::Stdio};

use tokio::process::Command as TokioCommand;

use crate::{
    HLS_STREAM_LOGGER_DOMAIN, cache::transcoding::HlsConfig, info_log,
};

pub async fn transmux_to_hls_vod(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<tokio::process::Child, String> {
    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let manifest_path = output_dir.join("master.m3u8");
    let input_str = input_path.to_str().ok_or("Invalid input path")?;
    let output_str = manifest_path.to_str().ok_or("Invalid output path")?;

    let segment_filename_str = output_dir
        .join("segment%05d.ts")
        .to_str()
        .ok_or("Invalid segment path")?
        .to_string();

    info_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Starting HLS VOD transcoding for: {}",
        input_str
    );

    let mut command = TokioCommand::new("ffmpeg");
    command
        .arg("-i")
        .arg(input_str)
        .arg("-map")
        .arg("0:v?")
        .arg("-map")
        .arg("0:a?")
        .arg("-c")
        .arg("copy")
        .arg("-f")
        .arg("hls")
        .arg("-hls_time")
        .arg(config.segment_duration_seconds.to_string())
        .arg("-hls_playlist_type")
        .arg("vod")
        .arg("-hls_segment_filename")
        .arg(&segment_filename_str)
        .arg(output_str)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let child = command.spawn().map_err(|e| e.to_string())?;

    Ok(child)
}
