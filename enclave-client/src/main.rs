extern crate alloc;

mod attestation;
mod runc;
mod runner_manager;

use std::path::PathBuf;

use crate::runner_manager::RunnerMessage;
use clap::Parser;
use common::messages::{EnclaveToHostMessage, HostToEnclaveMessage, Message};
use common::{init_tracing, protocol, short_wait, RunnerStartMode};
use futures::StreamExt as _;
use runner_manager::DirectRunnerManager;
use tokio::task;
use tokio_vsock::VsockListener;
use tracing::{debug, error, info};

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    /// The vsock address to listen on.
    pub vsock: String,
}

/// We model the client state as a typed state machine. The state transitions are driven by the
/// messages received from the host. We enforce that we never retrospectively can update protected
/// information such as the commit hash.
enum EnclaveState {
    Initializing,
    ReceivedStartMessage,
    Configured,
    WithMeasuredInput {
        commit_hash: String,
    },
    BuildFinished {
        commit_hash: String,
        artifact_name: String,
        artifact_hash: String,
        local_input_log_path: PathBuf,
    },
    Error,
}

impl EnclaveState {
    fn new() -> EnclaveState {
        EnclaveState::Initializing
    }

    fn on_start_message(self) -> EnclaveState {
        match self {
            EnclaveState::Initializing => EnclaveState::ReceivedStartMessage,
            _ => EnclaveState::Error,
        }
    }

    fn on_configured(self) -> EnclaveState {
        match self {
            EnclaveState::ReceivedStartMessage => EnclaveState::Configured,
            _ => EnclaveState::Error,
        }
    }

    fn on_received_commit_hash(self, commit_hash: String) -> EnclaveState {
        match self {
            EnclaveState::Configured => EnclaveState::WithMeasuredInput { commit_hash },
            _ => EnclaveState::Error,
        }
    }

    fn on_received_artifact(
        self,
        artifact_name: String,
        artifact_hash: String,
        local_input_log_path: PathBuf,
    ) -> EnclaveState {
        match self {
            EnclaveState::WithMeasuredInput { commit_hash } => EnclaveState::BuildFinished {
                commit_hash,
                artifact_name,
                artifact_hash,
                local_input_log_path,
            },
            _ => EnclaveState::Error,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    info!("Enclave client started");
    let mut enclave_state = EnclaveState::new();

    // init
    let args = Args::parse();
    init_tracing();
    debug!("{:?}", args);

    // listen to the vsock
    let addr = common::parse_vsock_addr(args.vsock)?;
    let listener = VsockListener::bind(addr)?;
    debug!("VsockListener bound: {}", addr);

    // we only accept one connection and then terminate
    let mut incoming = listener.incoming();
    let mut stream = incoming.next().await.expect("no incoming connection")?;
    info!("Accepted connection: {:?}", stream);

    // parse the initial message with the runner arguments
    let message = protocol::read_next_message(&mut stream).await?;
    let Message::HostToEnclave(HostToEnclaveMessage::StartRunner {
        enclave_client_args,
    }) = message
    else {
        anyhow::bail!("unexpected message: {:?}", message);
    };
    enclave_state = enclave_state.on_start_message();
    debug!("Received the enclave client args: {}", enclave_client_args);

    // Create and start the runner manager which babysits the GitHub Action Runner either as
    // a direct sub process or in a sandbox (using runc).
    let (runner_message_tx, mut runner_message_rx) = tokio::sync::mpsc::channel(32);
    let runner_manager_join_handle = match enclave_client_args.runner_start_mode {
        RunnerStartMode::Direct => {
            let runner_manager = DirectRunnerManager::new(
                enclave_client_args.fake_runner_args,
                enclave_client_args.runner_args.runner_version.clone(),
            )?;
            task::spawn(async move {
                if let Err(e) = runner_manager
                    .run(enclave_client_args.runner_args, runner_message_tx)
                    .await
                {
                    error!("Error running the runner: {:?}", e);
                }
            })
        }
        RunnerStartMode::Sandbox | RunnerStartMode::SandboxPlus => {
            let runner_manager = runner_manager::SandboxRunnerManager::new(
                enclave_client_args.fake_runner_args,
                enclave_client_args.runner_args.runner_version.clone(),
            )?;
            task::spawn(async move {
                if let Err(e) = runner_manager
                    .run(
                        enclave_client_args.runner_args,
                        runner_message_tx,
                        enclave_client_args.runner_start_mode,
                    )
                    .await
                {
                    error!("Error running the runner: {:?}", e);
                }
            })
        }
    };

    while let Some(runner_message) = runner_message_rx.recv().await {
        match runner_message {
            RunnerMessage::ConfigurationComplete => {
                enclave_state = enclave_state.on_configured();

                let message = Message::EnclaveToHost(EnclaveToHostMessage::Ok { info: None });
                protocol::write_message(&mut stream, &message).await?;
            }

            RunnerMessage::CommitHash { commit_hash } => {
                enclave_state = enclave_state.on_received_commit_hash(commit_hash.clone());

                let message = Message::EnclaveToHost(EnclaveToHostMessage::ReportRepositoryRoot {
                    commit_hash,
                });
                protocol::write_message(&mut stream, &message).await?;
            }

            RunnerMessage::ArtifactNameAndHash {
                artifact_name,
                artifact_hash,
                local_input_log_path,
            } => {
                enclave_state = enclave_state.on_received_artifact(
                    artifact_name.clone(),
                    artifact_hash.clone(),
                    local_input_log_path.clone(),
                );

                let message = Message::EnclaveToHost(EnclaveToHostMessage::ReportArtifact {
                    artifact_name,
                    artifact_hash,
                });
                protocol::write_message(&mut stream, &message).await?;
            }

            RunnerMessage::LogMessage { message } => {
                let message = Message::EnclaveToHost(EnclaveToHostMessage::Log { message });
                protocol::write_message(&mut stream, &message).await?;
            }

            RunnerMessage::TimestampMessage { marker, datetime } => {
                let message =
                    Message::EnclaveToHost(EnclaveToHostMessage::Timestamp { marker, datetime });
                protocol::write_message(&mut stream, &message).await?;
            }
        }

        match enclave_state {
            EnclaveState::BuildFinished {
                commit_hash,
                artifact_name,
                artifact_hash,
                local_input_log_path,
            } => {
                let attestation_document = attestation::perform_attestation(
                    enclave_client_args.use_fake_attestation,
                    commit_hash.clone(),
                    artifact_name.clone(),
                    artifact_hash.clone(),
                )
                .await?;
                let message = Message::EnclaveToHost(EnclaveToHostMessage::ReportAttestation {
                    attestation_document: attestation_document.clone(),
                });
                protocol::write_message(&mut stream, &message).await?;

                // TODO: handle the writing back a bit more elegantly..
                debug!(
                    "Writing the attestation document to {:?}",
                    local_input_log_path
                );
                std::fs::write(local_input_log_path, &attestation_document)?;

                break;
            }
            EnclaveState::Error => {
                anyhow::bail!("Enclave client state machine yields EnclaveState::Error");
            }
            _ => {}
        }
    }

    runner_manager_join_handle.await?;
    short_wait().await; // some time to take down the vsock connection

    info!("Enclave client finished");
    Ok(())
}
