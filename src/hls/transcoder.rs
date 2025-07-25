use std::{fs, path::Path};
use dashmap::DashMap;
use ffmpeg_next as ffmpeg;

use super::types::HlsConfig;

pub fn transmux_av_streams_to_hls(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<(), String> {
    ffmpeg::init().map_err(|e| e.to_string())?;
    ffmpeg::log::set_level(ffmpeg::log::Level::Error);

    let manifest_path = output_dir.join("master.m3u8");
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let mut ictx = ffmpeg::format::input(&input_path).map_err(|e| e.to_string())?;

    let hls_segment_filename = output_dir.join("segment%d_").to_str().unwrap().to_string();

    let mut opts = ffmpeg::Dictionary::new();
    opts.set("hls_time", &config.segment_duration_seconds.to_string());
    opts.set("hls_list_size", "0");
    opts.set("hls_segment_filename", &hls_segment_filename);
    opts.set("hls_flags", "independent_segments");
    opts.set("master_pl_name", "master.m3u8");


    let mut octx = ffmpeg::format::output_as_with(&manifest_path, "hls", opts)
        .map_err(|e| e.to_string())?;

    let stream_mapping = DashMap::new();
    let mut out_stream_index = 0;

    // --- Corrected Logic ---
    for in_stream in ictx.streams() {
        // Access parameters() to get stream info
        let medium = in_stream.parameters().medium();
        if medium == ffmpeg::media::Type::Video || medium == ffmpeg::media::Type::Audio {
            let mut out_stream = octx
                .add_stream(ffmpeg::codec::Id::None)
                .map_err(|e| e.to_string())?;
            out_stream.set_parameters(in_stream.parameters());
            unsafe {
                (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
            }
            stream_mapping.insert(in_stream.index(), out_stream_index);
            out_stream_index += 1;
        }
    }

    octx.write_header().map_err(|e| e.to_string())?;

    for (stream, mut packet) in ictx.packets() {
        if let Some(out_stream_idx) = stream_mapping.get(&stream.index()) {
            if let Some(out_stream) = octx.stream(*out_stream_idx) {
                packet.rescale_ts(stream.time_base(), out_stream.time_base());
                packet.set_stream(*out_stream_idx);
                packet.write_interleaved(&mut octx).map_err(|e| e.to_string())?;
            }
        }
    }

    octx.write_trailer().map_err(|e| e.to_string())?;
    Ok(())
}