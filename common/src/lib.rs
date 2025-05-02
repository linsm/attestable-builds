pub mod messages;
pub mod protocol;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use tokio::time;
use tokio_vsock::VsockAddr;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Arguments for configuring and starting the runner
#[derive(Clone, Serialize, Deserialize)]
pub struct RunnerArgs {
    /// The GitHub repository to run the actions on.
    pub github_repository: String,

    /// The registration token for the runner.
    pub github_reg_token: String,

    /// The PAT token for reading the repository (TODO: reduce in scope)
    pub github_pat_token: String,

    /// The runner version to use.
    pub runner_version: String,

    /// The user to start the runner as
    pub runner_user: String,

    /// The user's UID
    pub runner_uid: u32,

    /// The user's GID
    pub runner_gid: u32,
}

impl Display for RunnerArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RunnerArgs {{ github_repository: {}, github_reg_token: {} }}",
            self.github_repository,
            redact_token(&self.github_reg_token),
        )
    }
}

impl Debug for RunnerArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// Which mode the runner should be started in
#[derive(ValueEnum, Debug, Clone, Serialize, Deserialize)]
#[clap(rename_all = "snake_case")]
pub enum RunnerStartMode {
    /// The runner is called directly from the enclave client. It is initiated as a process,
    /// arguments are passed directly, and the output is captured via a local output.log file.
    Direct,

    /// The runner is initiated as a `runc` container. Arguments are passed via the `config.base.json`
    /// as ENV variables and the output is captured via a mounted `output.log` file.
    Sandbox,

    /// The runner is initiated as a `runsc` container using gvisor. Arguments are passed via the `config.base.json`
    /// as ENV variables and the output is captured via a mounted `output.log` file.
    SandboxPlus,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FakeRunnerArgs {
    /// The branch to check out (optional, defaults to main)
    pub branch_ref: Option<String>,

    /// The subproject dir to run
    pub subproject_dir: String,
}

/// Parses the fake runner arguments from a string. The string should be in the format
/// of `subproject_dir[@branch_ref]` where the second part is optional.
pub fn parse_fake_runner_args(s: String) -> anyhow::Result<FakeRunnerArgs> {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() == 1 {
        Ok(FakeRunnerArgs {
            branch_ref: None,
            subproject_dir: parts[0].to_string(),
        })
    } else {
        Ok(FakeRunnerArgs {
            branch_ref: Some(parts[1].to_string()),
            subproject_dir: parts[0].to_string(),
        })
    }
}

impl Display for FakeRunnerArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FakeRunnerArgs {{ subproject_dir: {}, branch_ref: {:?} }}",
            self.subproject_dir, self.branch_ref
        )
    }
}

/// Arguments for the enclave client that will start the runner.
#[derive(Clone, Serialize, Deserialize)]
pub struct EnclaveClientArgs {
    /// Arguments for the runner
    pub runner_args: RunnerArgs,

    /// Start mode of the runner
    pub runner_start_mode: RunnerStartMode,

    /// Whether to simulate the runner or not.
    pub fake_runner_args: Option<FakeRunnerArgs>,

    /// Whether to compute a fake attestation document (e.g. for running locally)
    pub use_fake_attestation: bool,
}

impl Display for EnclaveClientArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EnclaveClientArgs {{ runner_args: {}, runner_start_mode: {:?}, fake_runner_args: {}, use_fake_attestation: {} }}",
            self.runner_args,
            self.runner_start_mode,
            self.fake_runner_args.as_ref().map_or("None".to_string(), |args| args.to_string()),
            self.use_fake_attestation,
        )
    }
}

impl Debug for EnclaveClientArgs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// Initialize the tracing subscriber with the default settings. This will read the `RUST_LOG`
/// environment variable and set up the tracing subscriber accordingly. If the environment variable
/// is not set, it will default to `debug,tower_http=debug,axum::rejection=trace`.
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "debug,tower_http=debug,axum::rejection=trace,hyper_util=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

const VMADDR_CID_ANY: u32 = 0xFFFFFFFF;

/// Parse a vsock address from a string. The string should be in the format `CID:PORT`.
pub fn parse_vsock_addr(s: String) -> anyhow::Result<VsockAddr> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts[0] == "ANY" {
        Ok(VsockAddr::new(VMADDR_CID_ANY, parts[1].parse()?))
    } else {
        Ok(VsockAddr::new(parts[0].parse()?, parts[1].parse()?))
    }
}

/// A short wait function that waits for 100ms.
pub async fn short_wait() {
    time::sleep(time::Duration::from_millis(100)).await;
}

/// Redact a token by replacing all but the first and last characters with `*`. If the token is
/// less than 4 characters long, then all characters are replaced with `*`.
pub fn redact_token(token: &str) -> String {
    let len = token.len();
    if len < 4 {
        "*".repeat(len)
    } else {
        format!(
            "{}{}{}",
            &token[..1],
            "*".repeat(len - 2),
            &token[len - 1..]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vsock_addr() {
        let addr = parse_vsock_addr("1:2".to_string()).unwrap();
        assert_eq!(addr.cid(), 1);
        assert_eq!(addr.port(), 2);

        let addr = parse_vsock_addr("ANY:3".to_string()).unwrap();
        assert_eq!(addr.cid(), VMADDR_CID_ANY);
        assert_eq!(addr.port(), 3);
    }

    #[test]
    fn test_redact_token() {
        assert_eq!(redact_token("123456"), "1****6");
        assert_eq!(redact_token("123456789"), "1*******9");
        assert_eq!(redact_token("1234"), "1**4");
        assert_eq!(redact_token("123"), "***");
    }

    #[test]
    fn test_parse_fake_runner_args() {
        let args = parse_fake_runner_args("subproject".to_string()).unwrap();
        assert_eq!(args.branch_ref, None);
        assert_eq!(args.subproject_dir, "subproject");

        let args = parse_fake_runner_args("subproject@branch_ref".to_string()).unwrap();
        assert_eq!(args.branch_ref, Some("branch_ref".to_string()));
        assert_eq!(args.subproject_dir, "subproject");
    }
}
