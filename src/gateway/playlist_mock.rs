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

static PLAYLISTS_ANY: Lazy<Option<Regex>> =
    Lazy::new(|| compile_static_regex(r"(?i)^/(?:emby/)?Playlists(?:/|$)"));

static PLAYLISTS_BASE: Lazy<Option<Regex>> =
    Lazy::new(|| compile_static_regex(r"(?i)^/(?:emby/)?Playlists$"));

static PLAYLISTS_ITEMS: Lazy<Option<Regex>> =
    Lazy::new(|| compile_static_regex(r"(?i)^/(?:emby/)?Playlists/\w+/Items$"));

static PLAYLISTS_ITEM_SINGLE: Lazy<Option<Regex>> = Lazy::new(|| {
    compile_static_regex(r"(?i)^/(?:emby/)?Playlists/\w+/Items/\w+$")
});

static PLAYLISTS_ITEM_MOVE: Lazy<Option<Regex>> = Lazy::new(|| {
    compile_static_regex(r"(?i)^/(?:emby/)?Playlists/\w+/Items/\w+/Move/\w+$")
});

fn compile_static_regex(pattern: &str) -> Option<Regex> {
    match Regex::new(pattern) {
        Ok(regex) => Some(regex),
        Err(error) => {
            eprintln!("Failed to compile playlist regex '{pattern}': {error}");
            None
        }
    }
}

fn matches_regex(regex: &Lazy<Option<Regex>>, path: &str) -> bool {
    regex
        .as_ref()
        .is_some_and(|compiled| compiled.is_match(path))
}

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
        if *method == Method::POST && matches_regex(&PLAYLISTS_BASE, path) {
            return Some(ResponseBuilder::with_json(
                StatusCode::OK,
                r#"{"Id":"1000000000"}"#,
            ));
        }

        // POST /Playlists/{id}/Items → add items
        if *method == Method::POST && matches_regex(&PLAYLISTS_ITEMS, path) {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        // DELETE /Playlists/{id}/Items/{itemId} → remove item
        if *method == Method::DELETE
            && matches_regex(&PLAYLISTS_ITEM_SINGLE, path)
        {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        // GET /Playlists/{id}/Items → list items
        if *method == Method::GET && matches_regex(&PLAYLISTS_ITEMS, path) {
            return Some(ResponseBuilder::with_json(
                StatusCode::OK,
                r#"{"Items":[],"TotalRecordCount":0}"#,
            ));
        }

        // POST /Playlists/{id}/Items/{itemId}/Move/{target} → move item
        if *method == Method::POST && matches_regex(&PLAYLISTS_ITEM_MOVE, path)
        {
            return Some(ResponseBuilder::with_json(StatusCode::OK, ""));
        }

        if matches_regex(&PLAYLISTS_ANY, path) {
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
