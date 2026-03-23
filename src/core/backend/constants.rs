//! Shared literals for backend routing (avoid magic strings at call sites).

/// Substrings matched against `BackendNode.base_url` (lowercased) to treat the node as
/// «local» for legacy remote-URI assembly (`build_node_remote_uri`).
pub const LOCAL_NODE_HOST_MARKERS: &[&str] =
    &["127.0.0.1", "localhost", "0.0.0.0"];
