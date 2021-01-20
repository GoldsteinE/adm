use secstr::SecStr;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub repo_root: std::path::PathBuf,
    #[serde(deserialize_with = "deserialize_secstr")]
    pub webhook_secret: SecStr,
    #[serde(deserialize_with = "deserialize_opt_secstr")]
    pub telegram_token: Option<SecStr>,
    pub telegram_groups: Vec<i64>,
    pub parallel_builds: u8,
}

fn deserialize_secstr<'de, D>(de: D) -> Result<SecStr, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(de).map(|s| SecStr::new(s.into_bytes()))
}

fn deserialize_opt_secstr<'de, D>(de: D) -> Result<Option<SecStr>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(de).map(|o| o.map(|s| SecStr::new(s.into_bytes())))
}
