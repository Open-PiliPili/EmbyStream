use super::response::Response;
use crate::core::redirect_info::RedirectInfo;

pub enum Result {
    Stream(Response),
    Redirect(RedirectInfo),
}
