//! String constants for WebDav backend type and URL modes (no magic strings at call sites).

pub const BACKEND_TYPE: &str = "WebDav";

pub const MODE_PATH_JOIN: &str = "path_join";

pub const MODE_QUERY_PATH: &str = "query_path";

pub const MODE_URL_TEMPLATE: &str = "url_template";

pub const PROXY_MODE_ACCEL_REDIRECT: &str = "accel_redirect";

pub const TEMPLATE_PLACEHOLDER: &str = "{file_path}";

pub const DEFAULT_QUERY_PARAM: &str = "path";

pub const ACCEL_REDIRECT_HEADER: &str = "x-accel-redirect";

pub const ACCEL_REDIRECT_PREFIX: &str = "/_origin/webdav";
