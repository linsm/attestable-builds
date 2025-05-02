use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub commit_hash: String,
    pub artifact_hash: String,
    pub artifact_name: String,
    pub attestation_document: String,
}

impl LogEntry {
    pub fn to_byte_array(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];
        result.extend(self.commit_hash.as_bytes());
        result.extend(self.artifact_hash.as_bytes());
        result.extend(self.artifact_name.as_bytes());
        result.extend(self.attestation_document.as_bytes());
        result
    }

    pub fn to_merkle_hash(&self) -> String {
        let mut result: Vec<u8> = vec![];
        result.push(0);
        result.extend(self.to_byte_array());
        let digest = Sha256::new().chain_update(result).finalize()[..].to_vec();
        general_purpose::STANDARD.encode(digest)
    }
}
