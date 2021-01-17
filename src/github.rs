#[derive(Debug, Clone, serde::Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub owner: User,
    pub url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PushEvent {
    #[serde(rename = "ref")]
    pub reference: String,
    pub before: String,
    pub after: String,
    pub repository: Repository,
    pub sender: User,
}
