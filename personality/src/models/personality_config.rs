use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub trillian_url: String,
    pub admin_password: String,
    pub buildserver_password: String,
    pub token_secret: String,
}

impl PersonalityConfig {
    pub fn new(
        trillian_url: String,
        admin_password: String,
        buildserver_password: String,
        token_secret: String,
    ) -> Self {
        Self {
            trillian_url,
            admin_password,
            buildserver_password,
            token_secret,
        }
    }
}
