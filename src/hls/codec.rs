use std::{path::Path, process::Stdio};

use tokio::process::{Child, Command};

use crate::{
    HLS_STREAM_LOGGER_DOMAIN, cache::transcoding::HlsConfig, debug_log,
    info_log,
};

pub async fn transmux_to_hls_segments(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<Child, String> {
    let segment_filename_str = output_dir
        .join("segment%05d.ts")
        .to_str()
        .ok_or("Invalid segment path")?
        .to_string();

    info_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Starting HLS VOD transcoding for: {:?}",
        input_path
    );

    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-i")
        .arg(input_path.to_str().unwrap())
        .arg("-map")
        .arg("0:v:0?")
        .arg("-map")
        .arg("0:a:0?")
        .arg("-c")
        .arg("copy")
        .arg("-f")
        .arg("segment")
        .arg("-segment_time")
        .arg(config.segment_duration_seconds.to_string())
        .arg("-segment_format")
        .arg("mpegts")
        .arg("-segment_list_size")
        .arg("0")
        .arg(&segment_filename_str)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let command_string = format!(
        "Executing FFmpeg: {} {}",
        command.as_std().get_program().to_string_lossy(),
        command
            .as_std()
            .get_args()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    debug_log!(HLS_STREAM_LOGGER_DOMAIN, command_string);

    let child = command.spawn().map_err(|e| e.to_string())?;

    Ok(child)
}
