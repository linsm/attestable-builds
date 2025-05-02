use anyhow::bail;
use common::{redact_token, EnclaveClientArgs, RunnerArgs, RunnerStartMode};
use serde::Deserialize;
use tracing::debug;

pub mod backend;
pub mod log_publishing_service;
pub mod webhook_service;
pub mod webhook_types;

#[derive(Debug)]
pub enum BackendCommand {
    Start { run_id: u32 },
    Stop { run_id: u32 },
}

pub async fn load_enclave_client_args(
    fake_runner_args: Option<String>,
    use_fake_attestation: bool,
    runner_start_mode: RunnerStartMode,
    runner_version: String,
) -> anyhow::Result<EnclaveClientArgs> {
    let github_repository = std::env::var("GITHUB_REPOSITORY")?;
    debug!("github_repository: {}", github_repository);

    let github_pat_token = std::env::var("GITHUB_PAT_TOKEN")?;
    debug!("github_pat_token: {}", redact_token(&github_pat_token));

    // request a fresh registration token for the runner
    let github_reg_token = get_registration_token(&github_repository, &github_pat_token).await?;
    debug!("github_reg_token: {}", redact_token(&github_reg_token));

    let runner_user = std::env::var("RUNNER_USER")?;
    let runner_uid = std::env::var("RUNNER_UID")?.parse()?;
    let runner_gid = std::env::var("RUNNER_GID")?.parse()?;
    debug!(
        "runner_user: {} ({}:{})",
        runner_user, runner_uid, runner_gid
    );

    let fake_runner_args = match fake_runner_args {
        Some(s) => Some(common::parse_fake_runner_args(s)?),
        None => None,
    };

    Ok(EnclaveClientArgs {
        runner_args: RunnerArgs {
            github_repository,
            github_reg_token,
            github_pat_token,
            runner_version,
            runner_user,
            runner_uid,
            runner_gid,
        },
        runner_start_mode,
        fake_runner_args,
        use_fake_attestation,
    })
}

#[derive(Debug, Deserialize)]
pub struct GithubRegistrationTokenResponse {
    token: String,
}

async fn get_registration_token(
    github_repository: &str,
    github_pat_token: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "https://api.github.com/repos/{}/actions/runners/registration-token",
        github_repository
    );
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("User-Agent", "action-squares client")
        .header("Authorization", format!("Bearer {}", github_pat_token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !response.status().is_success() {
        bail!(
            "Failed to get a registration token: {:?} {:?}",
            response.status(),
            response.text().await?
        );
    }

    let json: GithubRegistrationTokenResponse = response.json().await?;
    Ok(json.token)
}
