use hyper::Uri;
use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(uri: &Option<Uri>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match uri {
        Some(u) => serializer.serialize_str(&u.to_string()),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uri>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) => s.parse::<Uri>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

pub fn serialize_uri_as_string<S>(uri: &Uri, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&uri.to_string())
}

pub fn deserialize_uri_from_string<'de, D>(deserializer: D) -> Result<Uri, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<Uri>().map_err(serde::de::Error::custom)
}
