use std::fmt;

use serde::Deserialize;

use crate::util::privacy::Privacy;

/// Configuration for the AList backend.
#[derive(Deserialize, Clone, Debug)]
pub struct AListConfig {
    pub base_url: String,
    pub token: String,
    #[serde(default)]
    pub path_replace_rule_regex: String,
}

impl fmt::Display for AListConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let privacy = Privacy::new();
        write!(
            f,
            "AListConfig {{ base_url: {}, token: {}, path_replace_rule_regex: {} }}",
            self.base_url,
            privacy.desensitize(&self.token),
            self.path_replace_rule_regex
        )
    }
}