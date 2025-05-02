use crate::backend::shared::interact_with_enclave_client;
use crate::log_publishing_service::AttestationEntry;
use crate::BackendCommand;
use anyhow::Result;
use common::{short_wait, EnclaveClientArgs, RunnerStartMode};
use std::collections::HashMap;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task;
use tokio_vsock::VsockAddr;
use tracing::{debug, error, info};

const HOST_VSOCK_ADDR_FOR_PROXY: &str = "3:5000"; // Ubuntu is normally "2" while AWS is "3"
const NITRO_IMAGE_NAME_STAMP: &str = "enclave.eif"; // with the sandbox
const NITRO_IMAGE_NAME_WET: &str = "enclave-wet.eif"; // without the sandbox but the GitHub runer
const ENCLAVE_CLIENT_VSOCK_PORT: u32 = 11000; // keep in sync with `enclave-container/content/run.sh`
const NITRO_ENCLAVE_CID: u32 = 42; // TODO: choose dynamically

#[derive(Debug, Clone)]
struct NitroConfiguration {
    cpu_count: u32,
    memory_mib: u32,
}

#[derive(Debug, Clone)]
pub enum NitroSize {
    Small,
    Large,
}

impl NitroSize {
    fn configuration(&self) -> NitroConfiguration {
        match self {
            // Half the size of `m5a.2xlarge`
            NitroSize::Small => NitroConfiguration {
                cpu_count: 4,
                memory_mib: 16384, // 16 GiB
            },
            // Half the size of `m5a.8xlarge`
            NitroSize::Large => NitroConfiguration {
                cpu_count: 16,
                memory_mib: 62000, // larger allocations are unreliable
            },
        }
    }
}

pub struct NitroService {
    enclave_client_args: EnclaveClientArgs,
    nitro_size: NitroSize,
    backend_command_rx: Receiver<BackendCommand>,
    log_entry_tx: Sender<AttestationEntry>,
    active_enclaves: Mutex<HashMap<u32, Box<NitroClient>>>,

    #[allow(dead_code)]
    host_proxy: Child,
}

pub struct NitroClient {
    pub cid: u32,
    pub interaction_task: task::JoinHandle<()>,
}

impl NitroService {
    pub async fn new(
        enclave_client_args: EnclaveClientArgs,
        nitro_size: NitroSize,
        backend_command_rx: Receiver<BackendCommand>,
        log_entry_tx: Sender<AttestationEntry>,
    ) -> Result<Self> {
        let active_enclaves = Mutex::new(HashMap::new());
        let host_proxy = start_host_proxy().await?;

        Ok(Self {
            enclave_client_args,
            nitro_size,
            backend_command_rx,
            log_entry_tx,
            active_enclaves,
            host_proxy,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
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
                    let cid = NITRO_ENCLAVE_CID;
                    let enclave_client_args = self.enclave_client_args.clone();
                    let log_entry_tx = self.log_entry_tx.clone();

                    spawn_nitro_enclave_client(
                        cid,
                        &enclave_client_args.runner_start_mode,
                        &self.nitro_size,
                    )
                    .await?;
                    let interaction_task = task::spawn(async move {
                        debug!("Starting the interaction task with the enclave client");
                        let result = interact_with_enclave_client(
                            VsockAddr::new(cid, ENCLAVE_CLIENT_VSOCK_PORT),
                            enclave_client_args,
                            log_entry_tx,
                        )
                        .await;
                        if let Err(e) = result {
                            error!("Failed to interact with the enclave client: {:?}", e);
                        }
                    });

                    let enclave_client = NitroClient {
                        cid,
                        interaction_task,
                    };
                    self.active_enclaves
                        .lock()
                        .await
                        .insert(run_id, Box::new(enclave_client));
                }

                BackendCommand::Stop { run_id } => {
                    if let Some(enclave_client) = self.active_enclaves.lock().await.remove(&run_id)
                    {
                        enclave_client.interaction_task.abort();
                        terminate_nitro_enclave(enclave_client.cid).await?;
                    }
                }
            }
        }
    }
}

pub async fn start_host_proxy() -> Result<Child> {
    let mut host_proxy = Command::new("./third-party/vsock-to-ip-transparent")
        .arg("--vsock-addr")
        .arg(HOST_VSOCK_ADDR_FOR_PROXY)
        .spawn()?;
    debug!("Spawned the host proxy: {:?}", host_proxy);

    // Wait 500ms (to give the proxy a chance to crash if it feels inclined to do so)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if host_proxy
        .try_wait()?
        .is_some_and(|status| !status.success())
    {
        error!("Host proxy failed to start");
        anyhow::bail!("Host proxy failed to start");
    } else {
        debug!("Host proxy still good after 500ms");
    }

    Ok(host_proxy)
}

pub async fn spawn_nitro_enclave_client(
    cid: u32,
    runner_start_mode: &RunnerStartMode,
    nitro_size: &NitroSize,
) -> Result<()> {
    let nitro_image_name = match runner_start_mode {
        RunnerStartMode::SandboxPlus => NITRO_IMAGE_NAME_STAMP,
        RunnerStartMode::Sandbox => NITRO_IMAGE_NAME_STAMP,
        RunnerStartMode::Direct => NITRO_IMAGE_NAME_WET,
    };
    let configuration = nitro_size.configuration();

    debug!(
        "Starting the Nitro enclave: {} {:?}",
        nitro_image_name, configuration
    );

    let output = Command::new("nitro-cli")
        .arg("run-enclave")
        .arg("--eif-path")
        .arg(nitro_image_name)
        .arg("--cpu-count")
        .arg(configuration.cpu_count.to_string())
        .arg("--memory")
        .arg(configuration.memory_mib.to_string())
        .arg("--enclave-cid")
        .arg(cid.to_string())
        .arg("--debug-mode")
        .output()
        .await?;

    if !output.status.success() {
        error!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Failed to run the Nitro enclave");
    } else {
        info!("Nitro enclave is running with cid={}", cid);
    }

    Ok(())
}

async fn terminate_nitro_enclave(cid: u32) -> Result<()> {
    let output = Command::new("nitro-cli")
        .arg("terminate-enclave")
        .arg("--enclave-id")
        .arg(cid.to_string())
        .output()
        .await?;

    if !output.status.success() {
        error!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Failed to terminate the Nitro enclave");
    } else {
        debug!("Nitro enclave is terminated with cid={}", cid);
    }

    Ok(())
}
