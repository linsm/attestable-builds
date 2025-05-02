use crate::log_publishing_service::AttestationEntry;
use common::messages::{
    create_new_timestamp_now, log_timestamp, EnclaveToHostMessage, HostToEnclaveMessage, Message,
};
use common::{protocol, EnclaveClientArgs};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time;
use tokio::time::{sleep, Instant};
use tokio_vsock::{VsockAddr, VsockStream};
use tracing::{debug, info, warn};

const ENCLAVE_CONNECTION_TIMEOUT_SECS: u64 = 60;

pub async fn interact_with_enclave_client(
    addr: VsockAddr,
    runner_args: EnclaveClientArgs,
    log_entry_tx: Sender<AttestationEntry>,
) -> anyhow::Result<()> {
    log_timestamp(&create_new_timestamp_now("ENCLAVE_STARTED"));
    debug!("Connecting to the enclave client on {:?}", addr);

    // minimal wait to allow the enclave client to start (helpful for local testing)
    sleep(Duration::from_millis(100)).await;

    // allow for multiple tries to connect to the enclave client
    let deadline = Instant::now() + Duration::from_secs(ENCLAVE_CONNECTION_TIMEOUT_SECS);
    let stream: anyhow::Result<VsockStream> = loop {
        match VsockStream::connect(addr).await {
            Ok(stream) => break Ok(stream),
            Err(e) => {
                warn!("Failed to connect to the enclave client: {:?}", e);
                time::sleep(Duration::from_secs(1)).await;

                if Instant::now() > deadline {
                    break Err(anyhow::anyhow!("Failed to connect to the enclave client"));
                }
            }
        }
    };
    let mut stream = stream?;
    info!("Connected to the enclave client");
    log_timestamp(&create_new_timestamp_now("ENCLAVE_CONNECTED"));

    // send runner args to the client
    let message = Message::HostToEnclave(HostToEnclaveMessage::StartRunner {
        enclave_client_args: runner_args,
    });
    protocol::write_message(&mut stream, &message).await?;
    debug!("Sent the runner args to the enclave client");

    // expect an OK message
    let message = protocol::read_next_message(&mut stream).await?;
    let Message::EnclaveToHost(EnclaveToHostMessage::Ok { info }) = message else {
        anyhow::bail!("Expected an OK message, got: {:?}", message);
    };
    debug!("Received an OK message: {:?}", info);
    log_timestamp(&create_new_timestamp_now("CONFIG_DONE"));

    // now we can start the main loop of interacting with the enclave client
    // we wait for a commit hash, an artifact report, and an attestation report (after which we end)
    // we might also get log and timestamp messages
    let mut maybe_commit_hash = None;
    let mut maybe_artifact_hash = None;
    let mut maybe_artifact_name = None;

    while let Ok(message) = protocol::read_next_message(&mut stream).await {
        match message {
            Message::EnclaveToHost(EnclaveToHostMessage::ReportRepositoryRoot { commit_hash }) => {
                maybe_commit_hash = Some(commit_hash.clone());
                debug!("Received the commit hash: {}", commit_hash);
            }
            Message::EnclaveToHost(EnclaveToHostMessage::ReportArtifact {
                artifact_hash,
                artifact_name,
            }) => {
                maybe_artifact_hash = Some(artifact_hash.clone());
                maybe_artifact_name = Some(artifact_name.clone());
                debug!(
                    "Received the artifact report: {} {}",
                    artifact_name, artifact_hash
                );
            }
            Message::EnclaveToHost(EnclaveToHostMessage::ReportAttestation {
                attestation_document,
            }) => {
                debug!(
                    "Received the attestation report: {:?}",
                    attestation_document
                );
                let commit_hash = maybe_commit_hash.take().expect("Missing commit hash");
                let artifact_hash = maybe_artifact_hash.take().expect("Missing artifact hash");
                let artifact_name = maybe_artifact_name.take().expect("Missing artifact name");

                let attestation_entry = AttestationEntry {
                    commit_hash,
                    artifact_hash,
                    artifact_name,
                    attestation_document,
                };
                log_entry_tx.send(attestation_entry).await?;
                break;
            }
            Message::EnclaveToHost(EnclaveToHostMessage::Log { message }) => {
                debug!("LOG: {}", message);
            }
            Message::EnclaveToHost(EnclaveToHostMessage::Timestamp { marker, datetime }) => {
                log_timestamp(&EnclaveToHostMessage::Timestamp { marker, datetime });
            }
            _ => {
                anyhow::bail!("Unexpected message: {:?}", message);
            }
        }
    }

    info!("Finished interacting with the enclave client");
    Ok(())
}
