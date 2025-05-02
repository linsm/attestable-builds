use base64::prelude::*;
use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProofModel {
    pub leaf_index: i64,
    pub hashes: Vec<String>,
    pub log_root: String,
}

impl ProofModel {
    pub fn new(leaf_index: i64, hashes_bytes: Vec<Vec<u8>>, signed_log_root: Vec<u8>) -> Self {
        let mut hashes: Vec<String> = vec![];
        for x in hashes_bytes {
            hashes.push(BASE64_STANDARD.encode(x));
        }
        let log_root = BASE64_STANDARD.encode(signed_log_root);
        Self {
            leaf_index,
            hashes,
            log_root,
        }
    }
}
