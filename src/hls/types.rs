use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct FfprobeOutput {
    pub format: FfprobeFormat,
}

#[derive(Deserialize, Debug)]
pub struct FfprobeFormat {
    #[serde(deserialize_with = "parse_duration")]
    pub duration: f64,
}

fn parse_duration<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = serde::Deserialize::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}
