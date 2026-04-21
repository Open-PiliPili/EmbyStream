use chrono::{Duration, Utc};
use serde::Deserialize;

use axum::{Json, Router, extract::State, routing::get};

use super::{
    api::WebAppState,
    contracts::{BackgroundItem, BackgroundProvider, LoginBackgroundResponse},
    error::WebError,
};

const BING_BASE_URL: &str = "https://www.bing.com";
const BING_ENDPOINT: &str = "https://www.bing.com/HPImageArchive.aspx";
const CACHE_KEY_LOGIN: &str = "login";
const CACHE_TTL_HOURS: i64 = 6;
const TMDB_ENDPOINT: &str = "https://api.themoviedb.org/3/trending/movie/day";
const TMDB_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p/original";

pub fn routes() -> Router<WebAppState> {
    Router::new().route("/login", get(login_background))
}

pub async fn login_background(
    State(state): State<WebAppState>,
) -> Result<Json<LoginBackgroundResponse>, WebError> {
    if let Some(cached) =
        state.db.load_background_cache(CACHE_KEY_LOGIN).await?
    {
        if cached.expires_at > Utc::now() {
            return Ok(Json(cached));
        }
    }

    let response = resolve_background_response(&state).await?;
    state
        .db
        .save_background_cache(CACHE_KEY_LOGIN, &response)
        .await?;

    Ok(Json(response))
}

async fn resolve_background_response(
    state: &WebAppState,
) -> Result<LoginBackgroundResponse, WebError> {
    if let Some(api_key) = state.config.tmdb_api_key.as_ref() {
        if let Ok(response) =
            fetch_tmdb_backgrounds(&state.http_client, api_key).await
        {
            return Ok(response);
        }
    }

    if let Ok(response) = fetch_bing_backgrounds(&state.http_client).await {
        return Ok(response);
    }

    Ok(static_fallback_background())
}

async fn fetch_tmdb_backgrounds(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<LoginBackgroundResponse, WebError> {
    let payload = client
        .get(TMDB_ENDPOINT)
        .query(&[("api_key", api_key), ("language", "en-US")])
        .send()
        .await
        .map_err(WebError::from)?
        .error_for_status()
        .map_err(WebError::from)?
        .json::<TmdbTrendingResponse>()
        .await
        .map_err(WebError::from)?;

    let items = payload
        .results
        .into_iter()
        .filter_map(|item| {
            item.backdrop_path
                .or(item.poster_path)
                .map(|path| BackgroundItem {
                    image_url: format!("{TMDB_IMAGE_BASE_URL}{path}"),
                    title: item.title,
                    subtitle: item.overview.filter(|value| !value.is_empty()),
                })
        })
        .take(8)
        .collect::<Vec<_>>();

    if items.is_empty() {
        return Err(WebError::internal(
            "TMDB did not return any usable background items.",
        ));
    }

    Ok(build_background_response(BackgroundProvider::Tmdb, items))
}

async fn fetch_bing_backgrounds(
    client: &reqwest::Client,
) -> Result<LoginBackgroundResponse, WebError> {
    let payload = client
        .get(BING_ENDPOINT)
        .query(&[("format", "js"), ("idx", "0"), ("n", "8"), ("mkt", "en-US")])
        .send()
        .await
        .map_err(WebError::from)?
        .error_for_status()
        .map_err(WebError::from)?
        .json::<BingArchiveResponse>()
        .await
        .map_err(WebError::from)?;

    let items = payload
        .images
        .into_iter()
        .map(|item| BackgroundItem {
            image_url: format!("{BING_BASE_URL}{}", item.url),
            title: item.title.unwrap_or(item.copyright),
            subtitle: item.copyright_link,
        })
        .take(8)
        .collect::<Vec<_>>();

    if items.is_empty() {
        return Err(WebError::internal(
            "Bing did not return any usable background items.",
        ));
    }

    Ok(build_background_response(BackgroundProvider::Bing, items))
}

fn static_fallback_background() -> LoginBackgroundResponse {
    let items = vec![
        BackgroundItem {
            image_url: "https://picsum.photos/id/1011/1600/900".to_string(),
            title: "Fallback Frame One".to_string(),
            subtitle: Some("Static fallback".to_string()),
        },
        BackgroundItem {
            image_url: "https://picsum.photos/id/1015/1600/900".to_string(),
            title: "Fallback Frame Two".to_string(),
            subtitle: Some("Static fallback".to_string()),
        },
        BackgroundItem {
            image_url: "https://picsum.photos/id/1025/1600/900".to_string(),
            title: "Fallback Frame Three".to_string(),
            subtitle: Some("Static fallback".to_string()),
        },
    ];

    build_background_response(BackgroundProvider::StaticFallback, items)
}

fn build_background_response(
    provider: BackgroundProvider,
    items: Vec<BackgroundItem>,
) -> LoginBackgroundResponse {
    let fetched_at = Utc::now();
    let expires_at = fetched_at + Duration::hours(CACHE_TTL_HOURS);

    LoginBackgroundResponse {
        provider,
        fetched_at,
        expires_at,
        items,
    }
}

#[derive(Debug, Deserialize)]
struct BingArchiveResponse {
    images: Vec<BingImageItem>,
}

#[derive(Debug, Deserialize)]
struct BingImageItem {
    url: String,
    #[serde(default)]
    title: Option<String>,
    copyright: String,
    #[serde(default)]
    copyright_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbTrendingResponse {
    results: Vec<TmdbMovieItem>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    overview: Option<String>,
    #[serde(default)]
    backdrop_path: Option<String>,
    #[serde(default)]
    poster_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        BackgroundProvider, build_background_response,
        static_fallback_background,
    };

    #[test]
    fn background_response_uses_six_hour_cache_window() {
        let response =
            build_background_response(BackgroundProvider::Bing, vec![]);
        let ttl = response.expires_at - response.fetched_at;
        assert_eq!(ttl.num_hours(), 6);
    }

    #[test]
    fn static_fallback_contains_items() {
        let response = static_fallback_background();
        assert_eq!(response.provider, BackgroundProvider::StaticFallback);
        assert!(!response.items.is_empty());
    }
}
