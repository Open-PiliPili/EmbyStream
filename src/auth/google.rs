use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use yup_oauth2::{
    ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
    authenticator_delegate::{
        DefaultInstalledFlowDelegate, InstalledFlowDelegate,
    },
};

const GOOGLE_AUTH_URI: &str = "https://accounts.google.com/o/oauth2/auth";
const GOOGLE_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DRIVE_READONLY_SCOPE: &str =
    "https://www.googleapis.com/auth/drive.readonly";

#[derive(Clone, Debug)]
pub struct GoogleAuthArgs {
    pub client_id: String,
    pub client_secret: String,
    pub no_browser: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct StoredTokenInfo {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<OffsetDateTime>,
}

#[derive(Clone)]
struct CliInstalledFlowDelegate {
    open_browser: bool,
}

impl InstalledFlowDelegate for CliInstalledFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            println!(
                "Open this URL to authorize Google Drive readonly access:"
            );
            println!("{url}");

            if self.open_browser {
                match webbrowser::open(url) {
                    Ok(_) => {
                        println!(
                            "Browser launch requested. If nothing opens, \
                             use the URL above manually."
                        );
                    }
                    Err(err) => {
                        println!("Browser launch failed: {err}");
                        println!(
                            "Continue by copying the URL into a browser manually."
                        );
                    }
                }
            } else {
                println!(
                    "Browser auto-open disabled. Copy the URL into a browser \
                     manually."
                );
            }

            DefaultInstalledFlowDelegate
                .present_user_url(url, need_code)
                .await
        })
    }
}

fn build_secret(client_id: &str, client_secret: &str) -> ApplicationSecret {
    ApplicationSecret {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        auth_uri: GOOGLE_AUTH_URI.to_string(),
        token_uri: GOOGLE_TOKEN_URI.to_string(),
        redirect_uris: vec![
            "http://127.0.0.1".to_string(),
            "http://localhost".to_string(),
        ],
        ..Default::default()
    }
}

fn temp_token_cache_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "embystream_google_auth_{}.json",
        uuid::Uuid::new_v4()
    ))
}

fn read_token_info(path: &Path) -> Result<StoredTokenInfo> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read token cache file {}", path.display()))?;
    parse_token_info(&content)
        .with_context(|| format!("parse token cache file {}", path.display()))
}

fn parse_token_info(content: &str) -> Result<StoredTokenInfo> {
    let value: Value = serde_json::from_str(content)?;
    parse_token_info_value(&value)
}

fn parse_token_info_value(value: &Value) -> Result<StoredTokenInfo> {
    if let Some(object) = value.as_object() {
        return parse_token_info_object(object);
    }

    if let Some(entries) = value.as_array() {
        let Some(token_value) =
            entries.iter().find_map(|entry| entry.get("token"))
        else {
            return Err(anyhow!("missing token entry in array token cache"));
        };
        return parse_token_info_value(token_value);
    }

    Err(anyhow!("unsupported token cache JSON shape"))
}

fn parse_token_info_object(
    object: &serde_json::Map<String, Value>,
) -> Result<StoredTokenInfo> {
    let access_token = string_field(object, "access_token");
    let refresh_token = string_field(object, "refresh_token");
    let expires_at = object
        .get("expires_at")
        .map(parse_expires_at_value)
        .transpose()?;

    Ok(StoredTokenInfo {
        access_token,
        refresh_token,
        expires_at,
    })
}

fn string_field(
    object: &serde_json::Map<String, Value>,
    field_name: &str,
) -> Option<String> {
    object
        .get(field_name)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn parse_expires_at_value(value: &Value) -> Result<OffsetDateTime> {
    if let Some(raw) = value.as_str() {
        return OffsetDateTime::parse(raw, &Rfc3339)
            .context("parse expires_at RFC3339 string");
    }

    let parts = value
        .as_array()
        .ok_or_else(|| anyhow!("expires_at must be a string or array"))?;
    if parts.len() != 9 {
        return Err(anyhow!(
            "expires_at array must contain 9 elements, got {}",
            parts.len()
        ));
    }

    let year = integer_part(parts, 0, "year")? as i32;
    let ordinal = integer_part(parts, 1, "ordinal")? as u16;
    let hour = integer_part(parts, 2, "hour")? as u8;
    let minute = integer_part(parts, 3, "minute")? as u8;
    let second = integer_part(parts, 4, "second")? as u8;
    let nanosecond = integer_part(parts, 5, "nanosecond")? as u32;
    let offset_hour = integer_part(parts, 6, "offset_hour")? as i8;
    let offset_minute = integer_part(parts, 7, "offset_minute")? as i8;
    let offset_second = integer_part(parts, 8, "offset_second")? as i8;

    let date = Date::from_ordinal_date(year, ordinal)
        .context("build expires_at date")?;
    let time = Time::from_hms_nano(hour, minute, second, nanosecond)
        .context("build expires_at time")?;
    let offset = UtcOffset::from_hms(offset_hour, offset_minute, offset_second)
        .context("build expires_at offset")?;

    Ok(PrimitiveDateTime::new(date, time).assume_offset(offset))
}

fn integer_part(
    parts: &[Value],
    index: usize,
    field_name: &str,
) -> Result<i64> {
    parts.get(index).and_then(Value::as_i64).ok_or_else(|| {
        anyhow!("expires_at[{index}] ({field_name}) must be an integer")
    })
}

fn print_token_result(token: &StoredTokenInfo) -> Result<()> {
    let access_token = token
        .access_token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing access_token after Google auth"))?;
    let refresh_token = token
        .refresh_token
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing refresh_token after Google auth"))?;

    println!("Google OAuth succeeded.");
    println!("access_token={access_token}");
    println!("refresh_token={refresh_token}");

    if let Some(expires_at) = token.expires_at {
        let formatted = expires_at
            .format(&Rfc3339)
            .context("format expires_at as RFC3339")?;
        println!("expires_at={formatted}");
    }

    Ok(())
}

pub async fn run_google_auth(args: &GoogleAuthArgs) -> Result<()> {
    let cache_path = temp_token_cache_path();
    let secret = build_secret(&args.client_id, &args.client_secret);
    let delegate = CliInstalledFlowDelegate {
        open_browser: !args.no_browser,
    };

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(&cache_path)
    .flow_delegate(Box::new(delegate))
    .build()
    .await
    .context("build Google installed-flow authenticator")?;

    let scopes = [GOOGLE_DRIVE_READONLY_SCOPE];
    auth.token(&scopes)
        .await
        .context("perform Google OAuth installed flow")?;

    let token_info = read_token_info(&cache_path)?;
    let print_result = print_token_result(&token_info);
    let _ = fs::remove_file(&cache_path);
    print_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn build_secret_uses_google_endpoints_and_loopback_redirects() {
        let secret = build_secret("cid", "sec");
        assert_eq!(secret.client_id, "cid");
        assert_eq!(secret.client_secret, "sec");
        assert_eq!(secret.auth_uri, GOOGLE_AUTH_URI);
        assert_eq!(secret.token_uri, GOOGLE_TOKEN_URI);
        assert!(
            secret
                .redirect_uris
                .iter()
                .any(|uri| uri == "http://127.0.0.1")
        );
        assert!(
            secret
                .redirect_uris
                .iter()
                .any(|uri| uri == "http://localhost")
        );
    }

    #[test]
    fn parse_token_info_supports_flat_object_payload() {
        let content = r#"{
            "access_token": "access-flat",
            "refresh_token": "refresh-flat",
            "expires_at": "2026-04-03T07:42:38Z"
        }"#;

        let token =
            parse_token_info(content).expect("parse flat token payload");

        assert_eq!(token.access_token.as_deref(), Some("access-flat"));
        assert_eq!(token.refresh_token.as_deref(), Some("refresh-flat"));
        assert_eq!(token.expires_at, Some(datetime!(2026-04-03 07:42:38 UTC)));
    }

    #[test]
    fn parse_token_info_supports_yup_oauth2_cache_payload() {
        let content = r#"[
            {
                "scopes": [
                    "https://www.googleapis.com/auth/drive.readonly"
                ],
                "token": {
                    "access_token": "access-nested",
                    "refresh_token": "refresh-nested",
                    "expires_at": [2026,93,7,42,38,184613000,0,0,0],
                    "id_token": null
                }
            }
        ]"#;

        let token = parse_token_info(content)
            .expect("parse nested token cache payload");

        assert_eq!(token.access_token.as_deref(), Some("access-nested"));
        assert_eq!(token.refresh_token.as_deref(), Some("refresh-nested"));
        assert_eq!(
            token.expires_at,
            Some(datetime!(2026-04-03 07:42:38.184613 UTC))
        );
    }
}
