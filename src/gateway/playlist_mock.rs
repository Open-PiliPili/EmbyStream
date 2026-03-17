use async_trait::async_trait;
use hyper::{Method, Response, StatusCode, body::Incoming};
use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    chain::{Middleware, Next},
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
};
use crate::{PLAYLIST_MOCK_LOGGER_DOMAIN, debug_log, warn_log};

static PLAYLISTS_ANY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:emby/)?Playlists(?:/|$)").expect("Invalid regex")
});

static PLAYLISTS_BASE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:emby/)?Playlists$").expect("Invalid regex")
});

static PLAYLISTS_ITEMS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:emby/)?Playlists/\w+/Items$").expect("Invalid regex")
});

static PLAYLISTS_ITEM_SINGLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:emby/)?Playlists/\w+/Items/\w+$")
        .expect("Invalid regex")
});

static PLAYLISTS_ITEM_MOVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^/(?:emby/)?Playlists/\w+/Items/\w+/Move/\w+$")
        .expect("Invalid regex")
});

#[derive(Clone)]
pub struct PlaylistMockMiddleware;

#[async_trait]
impl Middleware for PlaylistMockMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(
            PLAYLIST_MOCK_LOGGER_DOMAIN,
            "Starting playlist mock middleware..."
        );

        if let Some(response) = self.try_mock(&ctx) {
            debug_log!(
                PLAYLIST_MOCK_LOGGER_DOMAIN,
                "Mocked playlist response for {} {}",
                ctx.method,
                ctx.path
            );
            return response;
        }

        next(ctx, body).await
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

impl PlaylistMockMiddleware {
    fn try_mock(&self, ctx: &Context) -> Option<Response<BoxBodyType>> {
        let path = &ctx.path;
        let method = &ctx.method;

        // POST /Playlists → create playlist
        if *method == Method::POST && PLAYLISTS_BASE.is_match(path) {
            return Some(ResponseBuilder::with_json(
                StatusCode::OK,
                r#"{"Id":"1000000000"}"#,
            ));
        }

        // POST /Playlists/{id}/Items → add items
        if *method == Method::POST && PLAYLISTS_ITEMS.is_match(path) {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        // DELETE /Playlists/{id}/Items/{itemId} → remove item
        if *method == Method::DELETE && PLAYLISTS_ITEM_SINGLE.is_match(path) {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        // GET /Playlists/{id}/Items → list items
        if *method == Method::GET && PLAYLISTS_ITEMS.is_match(path) {
            return Some(ResponseBuilder::with_json(
                StatusCode::OK,
                r#"{"Items":[],"TotalRecordCount":0}"#,
            ));
        }

        // POST /Playlists/{id}/Items/{itemId}/Move/{target} → move item
        if *method == Method::POST && PLAYLISTS_ITEM_MOVE.is_match(path) {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        if PLAYLISTS_ANY.is_match(path) {
            warn_log!(
                PLAYLIST_MOCK_LOGGER_DOMAIN,
                "Unmatched playlist request: {} {} → 403",
                method,
                path
            );
            return Some(ResponseBuilder::with_status_code(
                StatusCode::FORBIDDEN,
            ));
        }

        None
    }
}
