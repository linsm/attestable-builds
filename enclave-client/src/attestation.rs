use anyhow::Ok;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use nsm_io::{Request, Response};
use serde_bytes::ByteBuf;
use tracing::{debug, warn};

pub async fn perform_attestation(
    use_fake_attestation: bool,
    commit_hash: String,
    artifact_name: String,
    artifact_hash: String,
) -> anyhow::Result<String> {
    if use_fake_attestation {
        perform_fake_attestation(commit_hash, artifact_name, artifact_hash).await
    } else {
        nitro_attestation(commit_hash, artifact_name, artifact_hash).await
    }
}

async fn nitro_attestation(
    commit_hash: String,
    artifact_name: String,
    artifact_hash: String,
) -> anyhow::Result<String> {
    let nsm_fd = nsm_driver::nsm_init();

    let user_data = ByteBuf::from(format!(
        "commit_hash={},artifact_name={},artifact_hash={}",
        commit_hash, artifact_name, artifact_hash
    ));

    // get pcr0-2 (also included in the attestation itself)
    let pcrs = vec![0, 1, 2];
    let mut pcr_values = vec![];
    for pcr in pcrs {
        let pcr_response = {
            let request = Request::DescribePCR { index: pcr };
            nsm_driver::nsm_process_request(nsm_fd, request)
        };
        let pcr_value = match pcr_response {
            Response::DescribePCR { lock: _, data } => data,
            _ => anyhow::bail!("Failed to get pcr{}", pcr),
        };
        debug!("pcr{}={}", pcr, BASE64_STANDARD.encode(&pcr_value));
        pcr_values.push(pcr_value);
    }

    // get attestation
    let attestation_response = {
        let request = Request::Attestation {
            user_data: Some(user_data),
            public_key: None,
            nonce: None,
        };
        nsm_driver::nsm_process_request(nsm_fd, request)
    };
    nsm_driver::nsm_exit(nsm_fd);

    let attestation_b64 = match attestation_response {
        Response::Attestation { document } => BASE64_STANDARD.encode(&document),
        _ => anyhow::bail!("Failed to get attestation document"),
    };

    let attestation_document = format!(
        r#"{{"commit_hash": "{}", "artifact_name": "{}", "artifact_hash": "{}", "pcr0": "{}", "pcr1": "{}", "pcr2": "{}", "attestation": "{}"}}"#,
        commit_hash,
        artifact_name,
        artifact_hash,
        BASE64_STANDARD.encode(&pcr_values[0]),
        BASE64_STANDARD.encode(&pcr_values[1]),
        BASE64_STANDARD.encode(&pcr_values[2]),
        attestation_b64
    );
    debug!("Attestation document: {}", attestation_document);
    Ok(attestation_document)
}

async fn perform_fake_attestation(
    commit_hash: String,
    artifact_name: String,
    artifact_hash: String,
) -> anyhow::Result<String> {
    warn!("Creating a fake attestation document");
    let attestation_document = format!(
        r#"{{"commit_hash": "{}", "artifact_name": "{}", "artifact_hash": "{}", "pcr0": "fake0", "pcr1": "fake1", "pcr2": "fake2", "attestation": "fake signature"}}"#,
        commit_hash, artifact_name, artifact_hash
    );
    debug!("Fake attestation document: {}", attestation_document);

    Ok(attestation_document)
}
