use std::{fs, path::Path};

use ffmpeg_next as ffmpeg;

use super::types::HlsConfig;

pub fn transmux_to_dash(
    input_path: &Path,
    output_dir: &Path,
    config: &HlsConfig,
) -> Result<(), String> {
    ffmpeg::init().map_err(|e| e.to_string())?;

    let manifest_path = output_dir.join("playlist.m3u8");
    fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let mut ictx =
        ffmpeg::format::input(&input_path).map_err(|e| e.to_string())?;

    let mut opts = ffmpeg::Dictionary::new();

    opts.set("hls_time", &config.segment_duration_seconds.to_string());
    opts.set("hls_list_size", "0");
    opts.set(
        "hls_segment_filename",
        output_dir.join("segment%05d.ts").to_str().unwrap(),
    );

    let mut octx = ffmpeg::format::output_as_with(&manifest_path, "hls", opts)
        .map_err(|e| e.to_string())?;

    for in_stream in ictx.streams() {
        let mut out_stream = octx
            .add_stream(ffmpeg::codec::Id::None)
            .map_err(|e| e.to_string())?;
        out_stream.set_parameters(in_stream.parameters());
        unsafe {
            (*out_stream.parameters().as_mut_ptr()).codec_tag = 0;
        }
    }

    octx.write_header().map_err(|e| e.to_string())?;

    for (stream, mut packet) in ictx.packets() {
        let out_stream_index = stream.index();
        if let Some(out_stream) = octx.stream(out_stream_index) {
            packet.rescale_ts(stream.time_base(), out_stream.time_base());
            packet.set_stream(out_stream_index);
            packet
                .write_interleaved(&mut octx)
                .map_err(|e| e.to_string())?;
        }
    }

    octx.write_trailer().map_err(|e| e.to_string())?;
    Ok(())
}
