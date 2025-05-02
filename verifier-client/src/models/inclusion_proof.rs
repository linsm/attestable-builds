use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct InclusionProof {
    pub leaf_index: i64,
    pub hashes: Vec<String>,
    pub log_root: String,
}
