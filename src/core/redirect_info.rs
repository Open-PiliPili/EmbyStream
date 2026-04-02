use hyper::{HeaderMap, Uri};

#[derive(Clone, Debug)]
pub struct RedirectInfo {
    pub target_url: Uri,
    pub final_headers: HeaderMap,
}

#[derive(Clone, Debug)]
pub struct AccelRedirectInfo {
    pub internal_path: String,
}
