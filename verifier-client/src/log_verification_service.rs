use crate::models::inclusion_proof::InclusionProof;
use base64::{engine::general_purpose, Engine as _};

use sha2::{Digest, Sha256};

pub async fn validate_inclusion_proof(
    tree_size: i64,
    merkle_hash: String,
    inclusion_proof: InclusionProof,
) -> anyhow::Result<()> {
    let local_root_node = calculate_root_node(
        inclusion_proof.leaf_index,
        merkle_hash,
        tree_size,
        inclusion_proof.hashes,
    );
    println!("Local root node: {}", local_root_node);
    let root_node_hash = get_root_node_from_signed_log_root(inclusion_proof.log_root.clone());
    println!("Signed root node: {}", root_node_hash);
    Ok(())
}

fn calculate_root_node(
    leaf_index: i64,
    merkle_hash: String,
    tree_size: i64,
    ordered_hashes: Vec<String>,
) -> String {
    let current_index = leaf_index;
    let inner = i64::BITS - (current_index ^ (tree_size - 1)).leading_zeros();

    let mut result = merkle_hash.clone();
    let mut left: String;
    let mut right: String;

    for i in 0..ordered_hashes.len() {
        if i < inner as usize && (((leaf_index >> i) & 1) == 0) {
            left = result.clone();
            right = ordered_hashes.get(i).unwrap().clone();
        } else {
            left = ordered_hashes.get(i).unwrap().clone();
            right = result.clone();
        }
        result = calculate_inner_node(left, right);
    }
    result
}

fn calculate_inner_node(left: String, right: String) -> String {
    let mut result: Vec<u8> = vec![];
    result.push(1);
    result.extend(general_purpose::STANDARD.decode(left).unwrap());
    result.extend(general_purpose::STANDARD.decode(right).unwrap());
    let digest = Sha256::new().chain_update(result).finalize()[..].to_vec();
    general_purpose::STANDARD.encode(digest)
}

fn get_root_node_from_signed_log_root(signed_log_root: String) -> String {
    let root_node_bytes = general_purpose::STANDARD.decode(signed_log_root).unwrap();
    let length = *root_node_bytes.get(10).unwrap();    
    let root_hash = root_node_bytes.get(11..11 + length as usize).unwrap();
    general_purpose::STANDARD.encode(root_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::inclusion_proof::InclusionProof;
    use crate::models::log_entry::LogEntry;

    #[tokio::test]
    async fn test_validate_inclusion_proof() {
        let log_entry = LogEntry {
            commit_hash: "commit-hash-test2".to_string(),
            artifact_hash: "artifact-hash-test".to_string(),
            artifact_name: "artifact-name-test".to_string(),
            attestation_document: "attestation-test-document".to_string(),
        };

        let inclusion_proof = InclusionProof {
        leaf_index: 1,
        hashes: vec!["kz/5DHcgmmecfKSbK7uQlJIc13jr8cTAU/d2hJ5WC80=".to_string(), "N418IioJ8s5bVW7gx4Nucmk8uAsHwaj+lrtMRs1uSGk=".to_string()],           
        log_root: "AAEAAAAAAAAAAyAfWovo4zFr6dnKIRBhY5KaHPWZeR2kvhMxDU00bZkSLRgyxL12w3WNAAAAAAAAAAAAAA==".to_string()
      };

        let local_root_node = calculate_root_node(
            inclusion_proof.leaf_index,
            log_entry.to_merkle_hash(),            
            3,
            inclusion_proof.hashes,
        );
        println!("Local root node: {}", local_root_node);
        assert!(local_root_node == "H1qL6OMxa+nZyiEQYWOSmhz1mXkdpL4TMQ1NNG2ZEi0=");

        let root_node_hash = get_root_node_from_signed_log_root(inclusion_proof.log_root.clone());
        println!("Signed root node: {}", root_node_hash);
        assert!(root_node_hash == local_root_node);
    }
}
