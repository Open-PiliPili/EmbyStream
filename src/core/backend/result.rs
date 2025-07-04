use super::{redirect_info::RedirectInfo, response::Response};

pub enum Result {
    Stream(Response),
    Redirect(RedirectInfo),
}
