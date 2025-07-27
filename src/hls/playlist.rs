use std::{fs, path::Path};

use tokio::process::Command;

use super::types::FfprobeOutput;
use crate::{HLS_STREAM_LOGGER_DOMAIN, debug_log, info_log};

pub async fn generate_m3u8_playlist(
    input_path: &Path,
    output_dir: &Path,
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
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .arg(input_str)
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffprobe. Is it installed and in your PATH? Error: {}", e))?;

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

    let mut playlist_content = String::new();
    playlist_content.push_str("#EXTM3U\n");
    playlist_content.push_str("#EXT-X-VERSION:3\n");

    let audio_group_id = "audio_group";
    let subtitle_group_id = "sub_group";
    let mut default_audio_set = false;

    let video_playlist_filename = "video.m3u8";

    for stream in &ffprobe_data.streams {
        if stream.codec_type == "audio" {
            let name: String = stream
                .tags
                .as_ref()
                .and_then(|t| t.title.as_deref())
                .map(String::from)
                .unwrap_or_else(|| format!("Track {}", stream.index));
            let lang = stream
                .tags
                .as_ref()
                .and_then(|t| t.language.as_deref())
                .unwrap_or("und");
            let default = if !default_audio_set {
                default_audio_set = true;
                "YES"
            } else {
                "NO"
            };

            playlist_content.push_str(&format!(
                "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"{}\",LANGUAGE=\"{}\",NAME=\"{}\",DEFAULT={},AUTOSELECT=YES,URI=\"{}\"\n",
                audio_group_id, lang, name, default, video_playlist_filename
            ));
        }
    }

    for stream in &ffprobe_data.streams {
        if stream.codec_type == "subtitle" {
            let name: String = stream
                .tags
                .as_ref()
                .and_then(|t| t.title.as_deref())
                .map(String::from)
                .unwrap_or_else(|| format!("Subtitle {}", stream.index));
            let lang = stream
                .tags
                .as_ref()
                .and_then(|t| t.language.as_deref())
                .unwrap_or("und");

            playlist_content.push_str(&format!(
                "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"{}\",LANGUAGE=\"{}\",NAME=\"{}\",DEFAULT=NO,AUTOSELECT=YES,URI=\"sub_{}.m3u8\"\n",
                subtitle_group_id, lang, name, stream.index
            ));
        }
    }

    let bandwidth =
        ffprobe_data.format.bit_rate.as_deref().unwrap_or("8000000");
    playlist_content.push_str(&format!(
        "#EXT-X-STREAM-INF:BANDWIDTH={},AUDIO=\"{}\",SUBTITLES=\"{}\"\n{}\n",
        bandwidth, audio_group_id, subtitle_group_id, video_playlist_filename
    ));

    let manifest_path = output_dir.join("master.m3u8");
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    fs::write(&manifest_path, playlist_content).map_err(|e| e.to_string())?;
    info_log!(
        HLS_STREAM_LOGGER_DOMAIN,
        "Instantly generated Master Playlist at: {:?}",
        manifest_path
    );

    Ok(())
}
