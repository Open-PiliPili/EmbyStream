use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct User {
    #[serde(rename = "Name", default)]
    pub name: Option<String>,
    #[serde(rename = "ServerId", default)]
    pub server_id: Option<String>,
    #[serde(rename = "Prefix", default)]
    pub prefix: Option<String>,
    #[serde(rename = "DateCreated", default)]
    pub date_created: Option<String>,
    #[serde(rename = "Id", default)]
    pub id: Option<String>,
    #[serde(rename = "HasPassword", default)]
    pub has_password: Option<bool>,
    #[serde(rename = "HasConfiguredPassword", default)]
    pub has_configured_password: Option<bool>,
    #[serde(rename = "LastLoginDate", default)]
    pub last_login_date: Option<String>,
    #[serde(rename = "LastActivityDate", default)]
    pub last_activity_date: Option<String>,
    #[serde(rename = "Configuration", default)]
    pub configuration: Option<UserConfiguration>,
    #[serde(rename = "Policy", default)]
    pub policy: Option<UserPolicy>,
    #[serde(rename = "HasConfiguredEasyPassword", default)]
    pub has_configured_easy_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserConfiguration {
    #[serde(rename = "AudioLanguagePreference", default)]
    pub audio_language_preference: Option<String>,
    #[serde(rename = "PlayDefaultAudioTrack", default)]
    pub play_default_audio_track: Option<bool>,
    #[serde(rename = "DisplayMissingEpisodes", default)]
    pub display_missing_episodes: Option<bool>,
    #[serde(rename = "SubtitleMode", default)]
    pub subtitle_mode: Option<String>,
    #[serde(rename = "OrderedViews", default)]
    pub ordered_views: Vec<String>,
    #[serde(rename = "LatestItemsExcludes", default)]
    pub latest_items_excludes: Vec<String>,
    #[serde(rename = "MyMediaExcludes", default)]
    pub my_media_excludes: Vec<String>,
    #[serde(rename = "HidePlayedInLatest", default)]
    pub hide_played_in_latest: Option<bool>,
    #[serde(rename = "HidePlayedInMoreLikeThis", default)]
    pub hide_played_in_more_like_this: Option<bool>,
    #[serde(rename = "HidePlayedInSuggestions", default)]
    pub hide_played_in_suggestions: Option<bool>,
    #[serde(rename = "RememberAudioSelections", default)]
    pub remember_audio_selections: Option<bool>,
    #[serde(rename = "RememberSubtitleSelections", default)]
    pub remember_subtitle_selections: Option<bool>,
    #[serde(rename = "EnableNextEpisodeAutoPlay", default)]
    pub enable_next_episode_auto_play: Option<bool>,
    #[serde(rename = "ResumeRewindSeconds", default)]
    pub resume_rewind_seconds: Option<i32>,
    #[serde(rename = "IntroSkipMode", default)]
    pub intro_skip_mode: Option<String>,
    #[serde(rename = "EnableLocalPassword", default)]
    pub enable_local_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserPolicy {
    #[serde(rename = "IsAdministrator", default)]
    pub is_administrator: Option<bool>,
    #[serde(rename = "IsHidden", default)]
    pub is_hidden: Option<bool>,
    #[serde(rename = "IsHiddenRemotely", default)]
    pub is_hidden_remotely: Option<bool>,
    #[serde(rename = "IsHiddenFromUnusedDevices", default)]
    pub is_hidden_from_unused_devices: Option<bool>,
    #[serde(rename = "IsDisabled", default)]
    pub is_disabled: Option<bool>,
    #[serde(rename = "LockedOutDate", default)]
    pub locked_out_date: Option<i64>,
    #[serde(rename = "AllowTagOrRating", default)]
    pub allow_tag_or_rating: Option<bool>,
    #[serde(rename = "BlockedTags", default)]
    pub blocked_tags: Vec<String>,
    #[serde(rename = "IsTagBlockingModeInclusive", default)]
    pub is_tag_blocking_mode_inclusive: Option<bool>,
    #[serde(rename = "IncludeTags", default)]
    pub include_tags: Vec<String>,
    #[serde(rename = "EnableUserPreferenceAccess", default)]
    pub enable_user_preference_access: Option<bool>,
    #[serde(rename = "AccessSchedules", default)]
    pub access_schedules: Vec<String>,
    #[serde(rename = "BlockUnratedItems", default)]
    pub block_unrated_items: Vec<String>,
    #[serde(rename = "EnableRemoteControlOfOtherUsers", default)]
    pub enable_remote_control_of_other_users: Option<bool>,
    #[serde(rename = "EnableSharedDeviceControl", default)]
    pub enable_shared_device_control: Option<bool>,
    #[serde(rename = "EnableRemoteAccess", default)]
    pub enable_remote_access: Option<bool>,
    #[serde(rename = "EnableLiveTvManagement", default)]
    pub enable_live_tv_management: Option<bool>,
    #[serde(rename = "EnableLiveTvAccess", default)]
    pub enable_live_tv_access: Option<bool>,
    #[serde(rename = "EnableMediaPlayback", default)]
    pub enable_media_playback: Option<bool>,
    #[serde(rename = "EnableAudioPlaybackTranscoding", default)]
    pub enable_audio_playback_transcoding: Option<bool>,
    #[serde(rename = "EnableVideoPlaybackTranscoding", default)]
    pub enable_video_playback_transcoding: Option<bool>,
    #[serde(rename = "EnablePlaybackRemuxing", default)]
    pub enable_playback_remuxing: Option<bool>,
    #[serde(rename = "EnableContentDeletion", default)]
    pub enable_content_deletion: Option<bool>,
    #[serde(rename = "RestrictedFeatures", default)]
    pub restricted_features: Vec<String>,
    #[serde(rename = "EnableContentDeletionFromFolders", default)]
    pub enable_content_deletion_from_folders: Vec<String>,
    #[serde(rename = "EnableContentDownloading", default)]
    pub enable_content_downloading: Option<bool>,
    #[serde(rename = "EnableSubtitleDownloading", default)]
    pub enable_subtitle_downloading: Option<bool>,
    #[serde(rename = "EnableSubtitleManagement", default)]
    pub enable_subtitle_management: Option<bool>,
    #[serde(rename = "EnableSyncTranscoding", default)]
    pub enable_sync_transcoding: Option<bool>,
    #[serde(rename = "EnableMediaConversion", default)]
    pub enable_media_conversion: Option<bool>,
    #[serde(rename = "EnabledChannels", default)]
    pub enabled_channels: Vec<String>,
    #[serde(rename = "EnableAllChannels", default)]
    pub enable_all_channels: Option<bool>,
    #[serde(rename = "EnabledFolders", default)]
    pub enabled_folders: Vec<String>,
    #[serde(rename = "EnableAllFolders", default)]
    pub enable_all_folders: Option<bool>,
    #[serde(rename = "InvalidLoginAttemptCount", default)]
    pub invalid_login_attempt_count: Option<i32>,
    #[serde(rename = "EnablePublicSharing", default)]
    pub enable_public_sharing: Option<bool>,
    #[serde(rename = "RemoteClientBitrateLimit", default)]
    pub remote_client_bitrate_limit: Option<i32>,
    #[serde(rename = "AuthenticationProviderId", default)]
    pub authentication_provider_id: Option<String>,
    #[serde(rename = "ExcludedSubFolders", default)]
    pub excluded_sub_folders: Vec<String>,
    #[serde(rename = "SimultaneousStreamLimit", default)]
    pub simultaneous_stream_limit: Option<i32>,
    #[serde(rename = "EnabledDevices", default)]
    pub enabled_devices: Vec<String>,
    #[serde(rename = "EnableAllDevices", default)]
    pub enable_all_devices: Option<bool>,
    #[serde(rename = "AllowCameraUpload", default)]
    pub allow_camera_upload: Option<bool>,
    #[serde(rename = "AllowSharingPersonalItems", default)]
    pub allow_sharing_personal_items: Option<bool>,
}
