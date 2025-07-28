use std::{fs, io, path::Path};

use hyper::Uri;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use reqwest::Url;
use thiserror::Error;

const PSEUDO_BASE_URI: &str = "http://local-file.invalid";

#[derive(Error, Debug)]
pub enum UriExtError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Invalid URI")]
    InvalidUri,
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub trait UriExt {
    fn from_path_or_url<S: AsRef<str>>(path: S) -> Result<Uri, UriExtError>;
    fn to_path_or_url_string(&self) -> String;

    fn is_local(&self) -> bool;
}

impl UriExt for Uri {
    fn from_path_or_url<S: AsRef<str>>(path: S) -> Result<Uri, UriExtError> {
        let path_str = path.as_ref();

        if path_str.starts_with("http://") || path_str.starts_with("https://") {
            let url = match Url::parse(path_str) {
                Ok(url) => url,
                Err(_) => return Err(UriExtError::InvalidUri),
            };

            return url.as_str().parse().map_err(|_| UriExtError::InvalidUri);
        }

        let path = Path::new(path_str);
        if !path.exists() {
            return Err(UriExtError::FileNotFound(path_str.to_string()));
        }
        let absolute_path = fs::canonicalize(path)?;
        let path_str = absolute_path.to_string_lossy();

        let normalized_path = if cfg!(windows) {
            path_str.replace('\\', "/")
        } else {
            path_str.into_owned()
        };

        let encoded_path =
            percent_encode(normalized_path.as_bytes(), NON_ALPHANUMERIC);
        let pseudo_uri = format!(
            "{}?path={}{}",
            PSEUDO_BASE_URI,
            if normalized_path.starts_with('/') {
                ""
            } else {
                "/"
            },
            encoded_path
        );

        pseudo_uri.parse().map_err(|_| UriExtError::InvalidUri)
    }

    fn to_path_or_url_string(&self) -> String {
        if !(self.scheme_str() == Some("http")
            && self.host() == Some("local-file.invalid"))
        {
            return self.to_string();
        }

        self.query()
            .and_then(|q| {
                form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k == "path")
                    .map(|(_, v)| {
                        let decoded = percent_encoding::percent_decode_str(&v)
                            .decode_utf8_lossy();
                        if cfg!(windows) {
                            decoded.replace('/', "\\")
                        } else {
                            decoded.into_owned()
                        }
                    })
            })
            .unwrap_or_else(|| self.to_string())
    }

    fn is_local(&self) -> bool {
        if let Some(scheme) = self.host() {
            return scheme.to_lowercase() == PSEUDO_BASE_URI;
        }

        self.host().is_none() && self.path().starts_with('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pathbuf() {
        let path = "****";

        let uri = Uri::from_path_or_url(path);
        if let Ok(uri) = uri {
            println!("uri -> {:?}", uri);
        } else {
            println!("uri -> {:?}", uri.unwrap_err());
        }
    }

    #[test]
    fn test_is_local() {
        let path = "****";

        let uri = Uri::from_path_or_url(path);
        let is_local = Uri::is_local(&uri.unwrap());
        println!("is_local -> {:?}", is_local);
    }
}
