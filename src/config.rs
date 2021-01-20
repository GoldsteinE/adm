use secstr::SecUtf8;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub repo_root: std::path::PathBuf,
    #[serde(deserialize_with = "deserialize_secutf8")]
    pub webhook_secret: SecUtf8,
    #[serde(deserialize_with = "deserialize_opt_secutf8")]
    pub telegram_token: Option<SecUtf8>,
    pub telegram_groups: Option<Vec<i64>>,
    pub parallel_builds: u8,
}

fn deserialize_secutf8<'de, D>(de: D) -> Result<SecUtf8, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(de).map(SecUtf8::from)
}

fn deserialize_opt_secutf8<'de, D>(de: D) -> Result<Option<SecUtf8>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(de).map(|o| o.map(SecUtf8::from))
}
