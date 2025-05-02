use backend::nitro::NitroSize;
use clap::{Parser, ValueEnum};
use common::messages::{create_new_timestamp_now, log_timestamp};
use common::RunnerStartMode;
use dotenv::dotenv;
use host_server::log_publishing_service::TransparencyLogConfiguration;
use host_server::{backend, webhook_service, BackendCommand};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::{task, time};
use tracing::{debug, info};

const CHANNEL_BUFFER_SIZE: usize = 10;

#[derive(ValueEnum, Debug, Clone)]
#[clap(rename_all = "snake_case")]
enum HostMode {
    Nitro,
    Local,
}

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    /// The mode in which the host server should run.
    mode: HostMode,

    /// Which runner start mode to use
    #[clap(long, default_value = "direct")]
    runner_start_mode: RunnerStartMode,

    /// The version of the action runner to use.
    #[clap(long, env = "RUNNER_VERSION")]
    runner_version: String,

    /// The base URL of the transparency log service. Defaults to localhost:8000.
    #[clap(long, env = "TRANSPARENCY_LOG_BASE_URL")]
    transparency_log_base_url: String,

    /// The username for the transparency log service. Will read from the `TRANSPARENCY_LOG_USERNAME` environment variable if not
    /// provided.
    #[clap(long, env = "TRANSPARENCY_LOG_USERNAME")]
    transparency_log_username: String,

    /// The password for the transparency log service. Will read from the `TRANSPARENCY_LOG_PASSWORD` environment variable if not
    /// provided.
    #[clap(long, env = "TRANSPARENCY_LOG_PASSWORD")]
    transparency_log_password: String,

    /// The log it of the specific tree.
    /// Endpoint to query available trees: /log/list-trees
    /// Will read from the `TRANSPARENCY_LOG_ID` environment variable if not
    /// provided.
    #[clap(long, env = "TRANSPARENCY_LOG_ID")]
    log_id: i64,

    /// Whether to simulate a job or not. If this is set, then the host server will not actually
    /// wait for a webhook event, but pretend that a job with if `42` has been started.
    #[clap(long, action)]
    simulate_webhook_event: bool,

    /// Whether the enclave client should use `github-runner/simulated` instead of the actual
    /// action runner version. The value provides the subproject_dir to use and the commit hash.
    /// The format is `subproject_dir[@branch_ref]` where the second part is optional.
    #[clap(long)]
    simulate_client_use_fake_runner: Option<String>,

    /// Whether the enclave client should use a fake attestation document.
    #[clap(long, action)]
    simulate_client_use_fake_attestation: bool,

    /// Whether the host should simulate the log publishing service.
    #[clap(long, action)]
    simulate_log_publishing: bool,

    /// For large jobs: this removes the timeout and uses the large enclave configuration.
    #[clap(long, action)]
    big_job: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    debug!("Loaded .env file");

    let args = Args::parse();
    common::init_tracing();
    debug!("{:?}", args);

    // Load service configurations
    let runner_args = host_server::load_enclave_client_args(
        args.simulate_client_use_fake_runner,
        args.simulate_client_use_fake_attestation,
        args.runner_start_mode,
        args.runner_version,
    )
    .await?;
    let transparency_log_config = TransparencyLogConfiguration {
        base_url: args.transparency_log_base_url,
        username: args.transparency_log_username,
        password: args.transparency_log_password,
        log_id: args.log_id,
        simulate: args.simulate_log_publishing,
    };

    // Start the log publishing service
    let (log_entry_tx, log_entry_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
    let join_handle_log_publishing = task::spawn(async move {
        host_server::log_publishing_service::run_log_publishing_service_blocking(
            transparency_log_config,
            log_entry_rx,
        )
        .await
        .expect("Log publishing service failed!");
    });

    // Start the respective backend service for the action runner
    let (backend_command_tx, backend_command_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
    let join_handle_backend = match args.mode {
        HostMode::Nitro => {
            let nitro_size = if args.big_job {
                NitroSize::Large
            } else {
                NitroSize::Small
            };
            // Spawn a new task that will run the Nitro service
            task::spawn(async move {
                let mut nitro_service = backend::nitro::NitroService::new(
                    runner_args,
                    nitro_size,
                    backend_command_rx,
                    log_entry_tx,
                )
                .await
                .expect("Failed to create Nitro service");
                nitro_service.run().await.expect("Nitro service failed");
            })
        }
        HostMode::Local => {
            // Spawn a new task that will run the enclave clients directly on the host
            task::spawn(async move {
                let mut local_service = backend::local::LocalService::new(
                    runner_args,
                    backend_command_rx,
                    log_entry_tx,
                );
                local_service.run().await.expect("Local service failed");
            })
        }
    };

    // Start either the webhook listener or simulate a job
    if args.simulate_webhook_event {
        let send_stop_command = !args.big_job;
        simulate_backend_trigger(&backend_command_tx, send_stop_command).await?;
    } else {
        webhook_service::run_webhook_service_blocking(backend_command_tx).await?;
    }

    join_handle_log_publishing.await?;
    join_handle_backend.await?;
    Ok(())
}

async fn simulate_backend_trigger(
    backend_command_tx: &Sender<BackendCommand>,
    send_stop_command: bool,
) -> anyhow::Result<()> {
    let simulated_job_id = 42;
    info!("Simulating a job with id={simulated_job_id}");

    log_timestamp(&create_new_timestamp_now("WEBHOOK"));
    backend_command_tx
        .send(BackendCommand::Start {
            run_id: simulated_job_id,
        })
        .await?;

    if send_stop_command {
        time::sleep(time::Duration::from_secs(600)).await;

        backend_command_tx
            .send(BackendCommand::Stop {
                run_id: simulated_job_id,
            })
            .await?;
    }

    Ok(())
}
