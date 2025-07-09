#[derive(Clone, Debug)]
pub struct ForwardInfo {
    pub item_id: String,
    pub media_source_id: String,
    pub path: String,
    pub token: String,
}

#[derive(Clone, Debug)]
pub struct PathParams {
    pub item_id: String,
}