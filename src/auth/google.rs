use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use anyhow::{Context, Result, anyhow};
use time::format_description::well_known::Rfc3339;
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
    expires_at: Option<time::OffsetDateTime>,
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
    serde_json::from_str(&content)
        .with_context(|| format!("parse token cache file {}", path.display()))
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
}
