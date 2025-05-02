use std::str;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttestationData {
    pub commit_hash: String,
    pub artifact_name: String,
    pub artifact_hash: String,
    pub pcr0: String,
    pub pcr1: String,
    pub pcr2: String,
    pub attestation_document: String
}