use crate::backend::shared::interact_with_enclave_client;
use crate::log_publishing_service::AttestationEntry;
use crate::BackendCommand;
use common::{short_wait, EnclaveClientArgs};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task;
use tokio_vsock::VsockAddr;
use tracing::{debug, error};

pub struct LocalService {
    runner_args: EnclaveClientArgs,
    backend_command_rx: Receiver<BackendCommand>,
    log_entry_tx: Sender<AttestationEntry>,
    active_children: Mutex<HashMap<u32, Box<LocalClient>>>,
}

pub struct LocalClient {
    pub process: Child,
    pub interaction_task: task::JoinHandle<()>,
}

/// The local service starts clients as processes on the same system, but still communicates with
/// them over vsock.
impl LocalService {
    pub fn new(
        runner_args: EnclaveClientArgs,
        backend_command_rx: Receiver<BackendCommand>,
        log_entry_tx: Sender<AttestationEntry>,
    ) -> Self {
        let active_children = Mutex::new(HashMap::new());
        Self {
            runner_args,
            backend_command_rx,
            log_entry_tx,
            active_children,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        debug!("Local service is running");
        loop {
            let maybe_command = self.backend_command_rx.try_recv();
            let Ok(command) = maybe_command else {
                short_wait().await;
                continue;
            };

            debug!("Received a command: {:?}", command);
            match command {
                BackendCommand::Start { run_id } => {
                    let port_id = run_id + 10000;
                    let runner_args = self.runner_args.clone();
                    let log_entry_tx = self.log_entry_tx.clone();

                    let process = spawn_local_client(port_id).await?;
                    let interaction_task = task::spawn(async move {
                        debug!("Starting the interaction task with the enclave client");
                        let result = interact_with_enclave_client(
                            VsockAddr::new(libc::VMADDR_CID_LOCAL, port_id),
                            runner_args,
                            log_entry_tx,
                        )
                        .await;
                        if let Err(e) = result {
                            error!("Failed to interact with the enclave client: {:?}", e);
                        }
                    });

                    let enclave_client = LocalClient {
                        process,
                        interaction_task,
                    };
                    self.active_children
                        .lock()
                        .await
                        .insert(run_id, Box::new(enclave_client));
                }

                BackendCommand::Stop { run_id } => {
                    if let Some(mut enclave_client) =
                        self.active_children.lock().await.remove(&run_id)
                    {
                        if enclave_client.process.try_wait()?.is_none() {
                            debug!(
                                "Killing the local enclave client process: {:?}",
                                enclave_client.process
                            );
                            enclave_client.interaction_task.abort();
                            enclave_client.process.kill().await?;
                        } else {
                            debug!("The local enclave client is already stopped");
                        }
                    }
                }
            }
        }
    }
}

pub async fn spawn_local_client(port_id: u32) -> anyhow::Result<Child> {
    let child = Command::new("target/debug/enclave-client")
        .arg(format!("1:{port_id}"))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    debug!("Spawned the local client: {:?}", child);
    Ok(child)
}
