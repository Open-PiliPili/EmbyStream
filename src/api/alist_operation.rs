/// Alist API operation enum.
#[derive(Debug, Clone)]
pub enum AlistOperation {
    /// Fetch file information for a given path.
    FsGet { path: String },
    /// Fetch link information for a given path.
    FsLink { path: String },
}
