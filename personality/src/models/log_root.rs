use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LogRoot {
    pub log_root: String,
}

impl LogRoot {}
