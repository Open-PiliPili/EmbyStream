use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct FfprobeOutput {
    pub streams: Vec<FfprobeStream>,
    pub format: FfprobeFormat,
}

#[derive(Deserialize, Debug)]
pub struct FfprobeStream {
    pub index: usize,
    pub codec_type: String,
    pub tags: Option<StreamTags>,
}

#[derive(Deserialize, Debug)]
pub struct StreamTags {
    pub language: Option<String>,
    pub title: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct FfprobeFormat {
    #[serde(deserialize_with = "parse_duration")]
    pub duration: f64,
    pub bit_rate: Option<String>,
    pub nb_streams: Option<usize>,
    pub size: Option<String>,
}

fn parse_duration<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}
