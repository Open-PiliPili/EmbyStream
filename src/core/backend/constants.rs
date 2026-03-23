//! Shared literals for backend routing (avoid magic strings at call sites).

/// Substrings matched against `BackendNode.base_url` (lowercased) to treat the node as
/// «local» for legacy remote-URI assembly (`build_node_remote_uri`).
pub const LOCAL_NODE_HOST_MARKERS: &[&str] =
    &["127.0.0.1", "localhost", "0.0.0.0"];

/// `BackendNode.type` value: local filesystem streaming only (no remote `base_url` relay).
pub const DISK_BACKEND_TYPE: &str = "Disk";

/// `BackendNode.type` value: relay signed streams to another host (`base_url` + query).
pub const STREAM_RELAY_BACKEND_TYPE: &str = "StreamRelay";

/// True when `base_url` is empty or whitespace only (after trim).
#[inline]
pub fn backend_base_url_is_empty(base_url: &str) -> bool {
    base_url.trim().is_empty()
}

/// True when `base_url` contains a loopback / placeholder host substring (see
/// [`LOCAL_NODE_HOST_MARKERS`]). Does **not** treat empty string as loopback; combine with
/// [`backend_base_url_is_empty`] at call sites when both semantics matter.
#[inline]
pub fn backend_base_url_is_local_host(base_url: &str) -> bool {
    let url = base_url.to_lowercase();
    LOCAL_NODE_HOST_MARKERS
        .iter()
        .any(|host| url.contains(host))
}
