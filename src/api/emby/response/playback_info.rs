use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
pub struct PlaybackInfo {
    #[serde(rename = "MediaSources", default)]
    pub media_sources: Vec<MediaSource>,
    #[serde(rename = "PlaySessionId", default)]
    pub play_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MediaSource {
    #[serde(rename = "Protocol", default)]
    pub protocol: Option<String>,
    #[serde(rename = "Id", default)]
    pub id: Option<String>,
    #[serde(rename = "Path", default)]
    pub path: Option<String>,
    #[serde(rename = "Type", default)]
    pub type_field: Option<String>,
    #[serde(rename = "Container", default)]
    pub container: Option<String>,
    #[serde(rename = "Size", default)]
    pub size: Option<i64>,
    #[serde(rename = "Name", default)]
    pub name: Option<String>,
    #[serde(rename = "IsRemote", default)]
    pub is_remote: Option<bool>,
    #[serde(rename = "HasMixedProtocols", default)]
    pub has_mixed_protocols: Option<bool>,
    #[serde(rename = "RunTimeTicks", default)]
    pub run_time_ticks: Option<i64>,
    #[serde(rename = "SupportsTranscoding", default)]
    pub supports_transcoding: Option<bool>,
    #[serde(rename = "SupportsDirectStream", default)]
    pub supports_direct_stream: Option<bool>,
    #[serde(rename = "SupportsDirectPlay", default)]
    pub supports_direct_play: Option<bool>,
    #[serde(rename = "IsInfiniteStream", default)]
    pub is_infinite_stream: Option<bool>,
    #[serde(rename = "RequiresOpening", default)]
    pub requires_opening: Option<bool>,
    #[serde(rename = "RequiresClosing", default)]
    pub requires_closing: Option<bool>,
    #[serde(rename = "RequiresLooping", default)]
    pub requires_looping: Option<bool>,
    #[serde(rename = "SupportsProbing", default)]
    pub supports_probing: Option<bool>,
    #[serde(rename = "MediaStreams", default)]
    pub media_streams: Vec<MediaStream>,
    #[serde(rename = "Formats", default)]
    pub formats: Vec<String>,
    #[serde(rename = "Bitrate", default)]
    pub bitrate: Option<i64>,
    #[serde(rename = "RequiredHttpHeaders", default)]
    pub required_http_headers: HashMap<String, String>,
    #[serde(rename = "AddApiKeyToDirectStreamUrl", default)]
    pub add_api_key_to_direct_stream_url: Option<bool>,
    #[serde(rename = "ReadAtNativeFramerate", default)]
    pub read_at_native_framerate: Option<bool>,
    #[serde(rename = "DefaultAudioStreamIndex", default)]
    pub default_audio_stream_index: Option<i32>,
    #[serde(rename = "ItemId", default)]
    pub item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MediaStream {
    #[serde(rename = "Codec", default)]
    pub codec: Option<String>,
    #[serde(rename = "CodecTag", default)]
    pub codec_tag: Option<String>,
    #[serde(rename = "Language", default)]
    pub language: Option<String>,
    #[serde(rename = "ColorTransfer", default)]
    pub color_transfer: Option<String>,
    #[serde(rename = "ColorPrimaries", default)]
    pub color_primaries: Option<String>,
    #[serde(rename = "ColorSpace", default)]
    pub color_space: Option<String>,
    #[serde(rename = "TimeBase", default)]
    pub time_base: Option<String>,
    #[serde(rename = "VideoRange", default)]
    pub video_range: Option<String>,
    #[serde(rename = "DisplayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "IsInterlaced", default)]
    pub is_interlaced: Option<bool>,
    #[serde(rename = "BitRate", default)]
    pub bit_rate: Option<i64>,
    #[serde(rename = "BitDepth", default)]
    pub bit_depth: Option<i32>,
    #[serde(rename = "RefFrames", default)]
    pub ref_frames: Option<i32>,
    #[serde(rename = "IsDefault", default)]
    pub is_default: Option<bool>,
    #[serde(rename = "IsForced", default)]
    pub is_forced: Option<bool>,
    #[serde(rename = "IsHearingImpaired", default)]
    pub is_hearing_impaired: Option<bool>,
    #[serde(rename = "Height", default)]
    pub height: Option<i32>,
    #[serde(rename = "Width", default)]
    pub width: Option<i32>,
    #[serde(rename = "AverageFrameRate", default)]
    pub average_frame_rate: Option<f32>,
    #[serde(rename = "RealFrameRate", default)]
    pub real_frame_rate: Option<f32>,
    #[serde(rename = "Profile", default)]
    pub profile: Option<String>,
    #[serde(rename = "Type", default)]
    pub type_field: Option<String>,
    #[serde(rename = "AspectRatio", default)]
    pub aspect_ratio: Option<String>,
    #[serde(rename = "Index", default)]
    pub index: Option<i32>,
    #[serde(rename = "IsExternal", default)]
    pub is_external: Option<bool>,
    #[serde(rename = "IsTextSubtitleStream", default)]
    pub is_text_subtitle_stream: Option<bool>,
    #[serde(rename = "SupportsExternalStream", default)]
    pub supports_external_stream: Option<bool>,
    #[serde(rename = "Protocol", default)]
    pub protocol: Option<String>,
    #[serde(rename = "PixelFormat", default)]
    pub pixel_format: Option<String>,
    #[serde(rename = "Level", default)]
    pub level: Option<i32>,
    #[serde(rename = "IsAnamorphic", default)]
    pub is_anamorphic: Option<bool>,
    #[serde(rename = "ExtendedVideoType", default)]
    pub extended_video_type: Option<String>,
    #[serde(rename = "ExtendedVideoSubType", default)]
    pub extended_video_sub_type: Option<String>,
    #[serde(rename = "ExtendedVideoSubTypeDescription", default)]
    pub extended_video_sub_type_description: Option<String>,
    #[serde(rename = "AttachmentSize", default)]
    pub attachment_size: Option<i32>,
    #[serde(rename = "ChannelLayout", default)]
    pub channel_layout: Option<String>,
    #[serde(rename = "Channels", default)]
    pub channels: Option<i32>,
    #[serde(rename = "SampleRate", default)]
    pub sample_rate: Option<i32>,
}