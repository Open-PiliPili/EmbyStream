use super::response::Response;
use crate::core::redirect_info::{AccelRedirectInfo, RedirectInfo};

pub enum Result {
    Stream(Response),
    Redirect(RedirectInfo),
    AccelRedirect(AccelRedirectInfo),
}
