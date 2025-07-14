use regex::{Error as RegexError, Regex};
use tokio::sync::OnceCell;

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
            Err(_) => path.to_string(),
        }
    }
}
