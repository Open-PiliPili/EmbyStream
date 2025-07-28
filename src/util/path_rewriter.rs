use regex::{Error as RegexError, Regex};
use tokio::sync::OnceCell;

use crate::{PATH_REWRITER_LOGGER_DOMAIN, error_log};

#[derive(Clone, Debug)]
pub struct PathRewriter {
    pattern: String,
    replacement: String,
    regex: OnceCell<Regex>,
}

impl PathRewriter {
    pub fn new(pattern: &str, replacement: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            replacement: replacement.to_string(),
            regex: OnceCell::new(),
        }
    }

    async fn compile(&self) -> Result<&Regex, RegexError> {
        self.regex
            .get_or_try_init(|| async { Regex::new(&self.pattern) })
            .await
    }

    pub async fn rewrite(&self, path: &str) -> String {
        match self.compile().await {
            Ok(re) => re.replace(path, &self.replacement).into_owned(),
            Err(e) => {
                error_log!(
                    PATH_REWRITER_LOGGER_DOMAIN,
                    "Error occurred during path rewrite: {:?}. ",
                    e
                );
                error_log!(
                    PATH_REWRITER_LOGGER_DOMAIN,
                    "Target path: {:?}, Pattern: {:?}, Regex: {:?}.",
                    path,
                    self.pattern,
                    self.replacement
                );
                path.to_string()
            }
        }
    }
}
