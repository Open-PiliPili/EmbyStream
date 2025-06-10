use std::collections::HashMap;

use serde_json::Value;

/// Represents different types of network request tasks that can be performed.
#[derive(Debug, Clone)]
pub enum NetworkTask {
    /// A plain text request without any specific data format
    RequestPlain,

    /// A JSON request containing structured data
    RequestJson(Value),

    /// A request with key-value parameters
    RequestParameters(HashMap<String, String>),

    /// A request with form data (multipart or URL-encoded)
    RequestMultipart(HashMap<String, String>),

    /// A request with form data including files (multipart/form-data)
    RequestMultipartWithFiles(HashMap<String, String>, Vec<(String, String)>),
}
