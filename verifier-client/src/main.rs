use clap::Parser;

use crate::models::log_entry::LogEntry;
use crate::models::attestation_data::AttestationData;
use dotenv::dotenv;

mod models;
mod transparency_service;
mod log_verification_service;
mod attestation_verification_service;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(long, default_value = "http://localhost:8090", env = "TRANSPARENCY_LOG_BASE_URL")]
    verifier_personality_base_url: String,

    #[clap(long)]
    verifier_tree_size: i64,

    #[clap(long, env = "VERIFIER_LOG_ID")]
    verifier_log_id: String,

    #[clap(long)]
    commit_hash: String,

    #[clap(long)]
    artifact_hash: String,

    #[clap(long)]
    artifact_name: String,

    #[clap(long)]
    pcr0: String,

    #[clap(long)]
    pcr1: String,

    #[clap(long)]
    pcr2: String,

    #[clap(long)]
    attestation_document: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let log_entry = LogEntry {
        commit_hash: args.commit_hash.to_string(),
        artifact_hash: args.artifact_hash.to_string(),
        artifact_name: args.artifact_name.to_string(),
        attestation_document: args.attestation_document.to_string(),
    };

    let result = transparency_service::request_inclusion_proof(
        args.verifier_personality_base_url,
        args.verifier_log_id,
        args.verifier_tree_size,
        log_entry.clone(),
    )
    .await?;
    log_verification_service::validate_inclusion_proof(
        args.verifier_tree_size,
        log_entry.clone().to_merkle_hash(),
        result,
    )
    .await?;

    let attestation_data = AttestationData {
        commit_hash: args.commit_hash.to_string(),
        artifact_name: args.artifact_name.to_string(),
        artifact_hash: args.artifact_hash.to_string(),
        pcr0: args.pcr0.to_string(),
        pcr1: args.pcr1.to_string(),
        pcr2: args.pcr2.to_string(),
        attestation_document: args.attestation_document.to_string(),
    };

    attestation_verification_service::validate_attestation_document(
        attestation_data.clone()).await?;

    Ok(())
}
