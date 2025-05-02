use crate::runc::{patch_config_json, ConfigJson, Mount, User};
use anyhow::anyhow;
use common::{FakeRunnerArgs, RunnerArgs, RunnerStartMode};
use std::path::{Path, PathBuf};
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;
use tokio::task::JoinHandle;
use tracing::field::debug;
use tracing::{debug, warn};

pub const RUNNER_NAME: &str = "NitroNorris";

pub enum RunnerMessage {
    ConfigurationComplete,
    CommitHash {
        commit_hash: String,
    },
    ArtifactNameAndHash {
        artifact_name: String,
        artifact_hash: String,
        local_input_log_path: PathBuf,
    },
    LogMessage {
        message: String,
    },
    TimestampMessage {
        marker: String,
        datetime: String,
    },
}

/**
 * The `DirectRunnerManager` runs the GitHub action runner directly as a sub process.
 */
pub(crate) struct DirectRunnerManager {
    fake_runner_args: Option<FakeRunnerArgs>,
    runner_path: PathBuf,
}

impl DirectRunnerManager {
    pub(crate) fn new(
        fake_runner_args: Option<FakeRunnerArgs>,
        runner_version: String,
    ) -> anyhow::Result<Self> {
        let base_path = std::env::current_dir()?;
        let runner_dir = build_runner_path(fake_runner_args.is_some(), runner_version);
        let runner_path = base_path.join(runner_dir);
        Ok(Self {
            fake_runner_args,
            runner_path,
        })
    }

    pub(crate) async fn run(
        self,
        runner_args: RunnerArgs,
        tx: Sender<RunnerMessage>,
    ) -> anyhow::Result<()> {
        // clean up any potential leftovers
        self.remove_runner_config().await?;

        // configure
        self.configure_runner(&runner_args).await?;
        tx.send(RunnerMessage::ConfigurationComplete).await?;

        let local_input_log_path = get_output_log_path(&self.runner_path)?.join("input.log");
        ensure_empty_input_log_file(&local_input_log_path).await?;

        // run everything in a separate task
        let (line_tx, mut line_rx) = mpsc::channel(32);
        let runner_task_handle = task::spawn(async move {
            if let Err(e) = self.run_runner(runner_args, line_tx).await {
                debug!("Error running the runner: {:?}", e);
            }
        });

        while let Some(line) = line_rx.recv().await {
            if let Some(message) = handle_incoming_log_message(&line, &local_input_log_path).await {
                tx.send(message).await?;
            }
        }

        runner_task_handle.await?;
        Ok(())
    }

    async fn remove_runner_config(&self) -> anyhow::Result<()> {
        let config_files = vec![
            ".runner",
            ".credentials",
            ".credentials_rsaparams",
            "svc.sh",
        ];

        for file in config_files {
            let path = self.runner_path.join(file);
            if path.exists() {
                debug!("Removing runner configuration file: {:?}", path);
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    async fn configure_runner(&self, runner_args: &RunnerArgs) -> anyhow::Result<()> {
        // configure the runner from fresh
        let exec = self.runner_path.join("config.sh");
        debug!("Configuring the runner: {:?}", exec);

        let mut command = Command::new("sudo");
        command
            .arg("-u")
            .arg(&runner_args.runner_user)
            .arg(exec)
            .arg("--url")
            .arg(format!(
                "https://github.com/{}",
                runner_args.github_repository
            ))
            .arg("--token")
            .arg(runner_args.github_reg_token.clone())
            .arg("--ephemeral")
            .arg("--disableupdate")
            .arg("--unattended")
            .arg("--replace")
            .arg("--name")
            .arg(RUNNER_NAME);

        add_fake_runner_env(&mut command, &self.fake_runner_args);

        let output = command.output().await?;

        if !output.status.success() {
            anyhow::bail!("Failed to configure the runner: {:?}", output);
        }

        Ok(())
    }

    async fn run_runner(
        self,
        runner_args: RunnerArgs,
        line_output: Sender<String>,
    ) -> anyhow::Result<()> {
        let exec = self.runner_path.join("run.sh");
        let hooks_dir = get_hooks_dir(&self.runner_path)?;
        let output_log_path = get_output_log_path(&self.runner_path)?.join("output.log");

        ensure_empty_output_log_file(&output_log_path).await?;

        debug!("Running the runner: {:?}", exec);
        let mut command = Command::new("sudo");
        command
            .env("LOG_HOOK", hooks_dir.join("log.sh"))
            .env("ATTESTATION_HOOK", hooks_dir.join("attestation.sh"))
            .env(
                "ACTIONS_RUNNER_HOOK_JOB_STARTED",
                hooks_dir.join("pre_hook.sh"),
            )
            .env("HOME", format!("/home/{}", runner_args.runner_user))
            .env("GITHUB_REPOSITORY", runner_args.github_repository)
            .env("GITHUB_PAT_TOKEN", runner_args.github_pat_token)
            .arg("--preserve-env")
            .arg("-u")
            .arg(&runner_args.runner_user)
            .arg(exec)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        add_fake_runner_env(&mut command, &self.fake_runner_args);

        let child = command.spawn()?;

        // start tailing the output log file while the child is running
        let tail_handle = spawn_file_tailer(line_output, &output_log_path);

        // wait for the runner to finish
        let output = child.wait_with_output().await?;
        tail_handle.abort();

        if !output.status.success() {
            warn!("Runner STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            warn!("Runner STDERR: {}", String::from_utf8_lossy(&output.stderr));
            anyhow::bail!("Failed to run the runner: {:?}", output.status);
        }

        Ok(())
    }
}

/**
 * The `SandboxRunnerManager` runs the GitHub action runner in a container using `runc`.
 */
#[derive(Debug)]
pub(crate) struct SandboxRunnerManager {
    sandbox_runner_path: PathBuf,
    local_output_path: PathBuf,
    local_output_log_path: PathBuf,
    local_input_log_path: PathBuf,
    local_sandbox_build_path: PathBuf,
    container_id: String,
    fake_runner_args: Option<FakeRunnerArgs>,
}

impl SandboxRunnerManager {
    pub(crate) fn new(
        fake_runner_args: Option<FakeRunnerArgs>,
        runner_version: String,
    ) -> anyhow::Result<Self> {
        let sandbox_base_path = PathBuf::from("/app/");
        let runner_dir = build_runner_path(fake_runner_args.is_some(), runner_version);
        let sandbox_runner_path = sandbox_base_path.join(runner_dir);

        let local_base_path = std::env::current_dir()?;

        // create a temporary directory for the output log relative to the current directory
        let local_output_path = local_base_path.join("tmp/output");
        if !local_output_path.exists() {
            std::fs::create_dir_all(&local_output_path)?;
        }
        let local_output_log_path = local_output_path.join("output.log");
        let local_input_log_path = local_output_path.join("input.log");

        // identify the build path for the sandbox
        let local_sandbox_build_path = local_base_path.join("sandbox-container/build");

        let result = Self {
            sandbox_runner_path,
            local_output_path,
            local_output_log_path,
            local_input_log_path,
            local_sandbox_build_path,
            container_id: "stampssandbox".to_string(),
            fake_runner_args,
        };
        debug!("SandboxRunnerManager: {:?}", &result);
        Ok(result)
    }

    pub(crate) async fn run(
        self,
        runner_args: RunnerArgs,
        tx: Sender<RunnerMessage>,
        runner_mode: RunnerStartMode,
    ) -> anyhow::Result<()> {
        // patch the config.base.json file
        let local_base_config_json_path = self.local_sandbox_build_path.join("config.base.json");
        let local_config_json_path = self.local_sandbox_build_path.join("config.json");
        self.patch_config_json(
            &local_base_config_json_path,
            &local_config_json_path,
            runner_args,
        )
        .await?;

        let program = match runner_mode {
            RunnerStartMode::Sandbox => "runc",
            RunnerStartMode::SandboxPlus => "runsc",
            _ => anyhow::bail!("Invalid runner mode for SandboxRunnerManager"),
        };

        // remove all container instances from runc that might be around
        let _ = Command::new(program)
            .arg("delete")
            .arg(self.container_id.clone())
            .output()
            .await?;
        debug!("Removed container if any (ignoring failures)");

        ensure_empty_output_log_file(&self.local_output_log_path).await?;

        let local_input_log_path = self.local_input_log_path.clone();
        ensure_empty_input_log_file(&local_input_log_path).await?;

        let (line_tx, mut line_rx) = mpsc::channel(32);
        let container_task_handle = task::spawn(async move {
            if let Err(e) = self.run_container(line_tx, program).await {
                debug!("Error running the container: {:?}", e);
            }
            debug("Container task finished");
        });

        while let Some(line) = line_rx.recv().await {
            if let Some(message) = handle_incoming_log_message(&line, &local_input_log_path).await {
                tx.send(message).await?;
            }
        }

        container_task_handle.await?;
        Ok(())
    }

    async fn run_container(&self, line_tx: Sender<String>, program: &str) -> anyhow::Result<()> {
        let running_container_child = Command::new(program)
            .arg("run")
            .arg("--bundle")
            .arg(&self.local_sandbox_build_path)
            .arg(self.container_id.clone())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        debug!("Started container");

        let tail_handle = spawn_file_tailer(line_tx, &self.local_output_log_path);

        // wait for the container to finish
        let container_result = running_container_child.wait_with_output().await?;
        tail_handle.abort();

        if !container_result.status.success() {
            warn!(
                "Container STDOUT: {}",
                String::from_utf8_lossy(&container_result.stdout)
            );
            warn!(
                "Container STDERR: {}",
                String::from_utf8_lossy(&container_result.stderr)
            );
            anyhow::bail!("Failed to run the container: {:?}", container_result.status);
        }

        Ok(())
    }

    async fn patch_config_json(
        &self,
        local_base_config_json_path: &PathBuf,
        local_config_json_path: &PathBuf,
        runner_args: RunnerArgs,
    ) -> anyhow::Result<()> {
        let sandbox_hooks_path = get_hooks_dir(&self.sandbox_runner_path)?;
        let sandbox_output_path = get_output_log_path(&self.sandbox_runner_path)?;

        // read the base config.base.json that we will then path
        let config_json_string = std::fs::read_to_string(local_base_config_json_path)?;
        let config_json: ConfigJson = serde_json::from_str(&config_json_string)?;

        // args: start the entry.sh using bash
        let args = Some("/bin/bash entry.sh".to_string());

        // env: pass in the values from the runner_args
        let mut env = vec![
            "ACTIONS_RUNNER_DEBUG=1".to_string(),
            format!("GITHUB_REPOSITORY={}", runner_args.github_repository),
            format!("GITHUB_REG_TOKEN={}", runner_args.github_reg_token),
            format!("GITHUB_PAT_TOKEN={}", runner_args.github_pat_token),
            format!(
                "GITHUB_RUNNER_PATH={}",
                &self.sandbox_runner_path.to_string_lossy().to_string()
            ),
            format!("GITHUB_RUNNER_NAME={}", RUNNER_NAME.to_string()),
            format!(
                "LOG_HOOK={}",
                sandbox_hooks_path.join("log.sh").to_string_lossy()
            ),
            format!(
                "ATTESTATION_HOOK={}",
                sandbox_hooks_path.join("attestation.sh").to_string_lossy()
            ),
            format!(
                "ACTIONS_RUNNER_HOOK_JOB_STARTED={}",
                sandbox_hooks_path.join("pre_hook.sh").to_string_lossy()
            ),
        ];

        // See method `add_fake_runner_env` for the motivation behind these extra env variables
        if let Some(fake_runner_args) = &self.fake_runner_args {
            if let Some(branch_ref) = &fake_runner_args.branch_ref {
                env.push(format!("GITHUB_REF_NAME={}", branch_ref));
            }
            env.push(format!(
                "SUBPROJECT_DIR={}",
                fake_runner_args.subproject_dir
            ));
        }

        let env = Some(env);

        // cwd: set to /app as per Dockerfile
        let cwd = Some("/app".to_string());

        // user: the user `runner` with the UID and GID from the runner_args
        let user = Some(User {
            uid: runner_args.runner_uid,
            gid: runner_args.runner_gid,
        });

        // additional_mounts: mount the output directory
        let mount_options = Some(vec!["rbind".to_string(), "rw".to_string()]);
        let additional_mounts = Some(vec![Mount {
            destination: sandbox_output_path.to_string_lossy().to_string(),
            type_: "none".to_string(),
            source: self.local_output_path.to_string_lossy().to_string(),
            options: mount_options,
        }]);

        // patch the config.base.json
        let patched_config_json =
            patch_config_json(config_json, args, env, user, cwd, additional_mounts);

        // and write it back
        let serialized = serde_json::to_string(&patched_config_json)?;
        std::fs::write(local_config_json_path, serialized)?;

        debug!(
            "Patched config.base.json: {:?} -> {:?}",
            local_base_config_json_path, local_config_json_path
        );

        Ok(())
    }
}

fn spawn_file_tailer(line_output: Sender<String>, output_log: &Path) -> JoinHandle<()> {
    let output_log = output_log.to_owned();
    let tail_handle = tokio::task::spawn(async move {
        let mut tail = tokio::process::Command::new("tail")
            .arg("-f")
            .arg(&output_log)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to start tail");

        let tail_stdout = tail.stdout.take().expect("Failed to get tail stdout");
        let mut tail_buf_reader = tokio::io::BufReader::new(tail_stdout);

        loop {
            let mut buf = String::new();
            tail_buf_reader
                .read_line(&mut buf)
                .await
                .expect("Failed to read line");
            line_output.send(buf).await.expect("Failed to send line");
        }
    });
    tail_handle
}

async fn handle_incoming_log_message(
    line: &str,
    local_input_log_path: &PathBuf,
) -> Option<RunnerMessage> {
    if line.starts_with("RUNNER_CONFIGURATION_DONE") {
        Some(RunnerMessage::ConfigurationComplete)
    } else if line.starts_with("RUNNER_FINISHED") {
        debug!("Runner finished");
        None
    } else if line.starts_with("GIT_HASH") {
        let commit_hash = extract_value_from_line(line).unwrap();
        Some(RunnerMessage::CommitHash { commit_hash })
    } else if line.starts_with("ARTIFACT_NAME_AND_HASH") {
        let artifact_name_and_hash = extract_value_from_line(line).unwrap();
        let (artifact_name, artifact_hash) = artifact_name_and_hash
            .split_once(';')
            .expect("bad ARTIFACT_NAME_AND_HASH format");
        Some(RunnerMessage::ArtifactNameAndHash {
            artifact_name: artifact_name.to_string(),
            artifact_hash: artifact_hash.to_string(),
            local_input_log_path: local_input_log_path.to_owned(),
        })
    } else if line.starts_with("LOG") {
        // message is everything after LOG
        let message = line.split_whitespace().nth(1).unwrap().to_string();
        Some(RunnerMessage::LogMessage { message })
    } else if line.starts_with("TIMESTAMP") {
        // starts with TIMESTAMP followed by a marker and a datetime (all separated by a space)
        let mut parts = line.split_whitespace();
        let marker = parts.nth(1).unwrap().to_string();
        let datetime = parts.next().unwrap().to_string();
        Some(RunnerMessage::TimestampMessage { marker, datetime })
    } else {
        None
    }
}

fn build_runner_path(use_fake_runner: bool, runner_version: String) -> PathBuf {
    let mut runner_path = PathBuf::from("github-runner");

    if use_fake_runner {
        runner_path.push("simulated");
    } else {
        runner_path.push(runner_version);
    }

    runner_path
}

fn get_hooks_dir(runner_path: &Path) -> anyhow::Result<PathBuf> {
    let path = runner_path
        .parent()
        .ok_or(anyhow!("no parent folder"))?
        .join("hooks");
    Ok(path)
}

fn get_output_log_path(runner_path: &Path) -> anyhow::Result<PathBuf> {
    let path = runner_path
        .parent()
        .ok_or(anyhow!("no parent folder"))?
        .join("output");
    Ok(path)
}

fn extract_value_from_line(line: &str) -> Option<String> {
    Some(line.split('=').nth(1).unwrap().trim().to_string())
}

async fn ensure_empty_output_log_file(local_output_log_path: &Path) -> anyhow::Result<()> {
    // replace any previous output file content
    std::fs::write(local_output_log_path, "")?;

    // change ownership to runner user
    let _ = tokio::process::Command::new("chown")
        .arg("runner:runner")
        .arg(local_output_log_path)
        .output()
        .await?;

    // change permission to so that runner can write and everyone can read
    let _ = tokio::process::Command::new("chmod")
        .arg("644")
        .arg(local_output_log_path)
        .output()
        .await?;
    Ok(())
}

async fn ensure_empty_input_log_file(local_input_log_path: &Path) -> anyhow::Result<()> {
    // replace any previous output file content
    std::fs::write(local_input_log_path, "")?;

    // change ownership to runner user
    let _ = tokio::process::Command::new("chown")
        .arg("runner:runner")
        .arg(local_input_log_path)
        .output()
        .await?;

    // change permission to so that everyone can read/write
    let _ = tokio::process::Command::new("chmod")
        .arg("666")
        .arg(local_input_log_path)
        .output()
        .await?;
    Ok(())
}

fn add_fake_runner_env(command: &mut Command, fake_runner_args: &Option<FakeRunnerArgs>) {
    if let Some(fake_runner_args) = fake_runner_args {
        if let Some(branch_ref) = &fake_runner_args.branch_ref {
            // this is typically injected by the GitHub Runner
            command.env("GITHUB_REF_NAME", branch_ref);
        }

        // this is not needed by the GitHub Runner as it is guided by the checkout action which
        // sets the working directory
        command.env("SUBPROJECT_DIR", &fake_runner_args.subproject_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::parse_fake_runner_args;

    #[test]
    fn test_extract_value_from_line() {
        let line = "GIT_HASH=123456";
        let value = extract_value_from_line(line).unwrap();
        assert_eq!(value, "123456");
    }

    #[test]
    fn test_extract_value_from_line_with_separator_in_value() {
        let line = "ARTIFACT_NAME_AND_HASH=artifact-name;artifact-hash";
        let value = extract_value_from_line(line).unwrap();
        assert_eq!(value, "artifact-name;artifact-hash");
    }

    #[test]
    fn test_extract_value_from_line_with_no_value() {
        let line = "GIT_HASH=";
        let value = extract_value_from_line(line).unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn test_paths_from_runner_path_with_version() {
        let runner_path = PathBuf::from("github-runner/2.278.0");
        let hooks_dir = get_hooks_dir(&runner_path).unwrap();
        let output_log_path = get_output_log_path(&runner_path).unwrap();
        assert_eq!(hooks_dir, PathBuf::from("github-runner/hooks"));
        assert_eq!(output_log_path, PathBuf::from("github-runner/output"));
    }

    #[test]
    fn test_build_runner_path_with_fake_runner() {
        let runner_path = build_runner_path(true, "1.234.0".to_string());
        assert_eq!(runner_path, PathBuf::from("github-runner/simulated"));
    }

    #[test]
    fn test_build_runner_path_with_real_runner() {
        let runner_path = build_runner_path(false, "1.234.0".to_string());
        assert_eq!(runner_path, PathBuf::from("github-runner/1.234.0"));
    }

    #[tokio::test]
    async fn test_add_fake_runner_env_with_commit_set() {
        let mut command = Command::new("env");
        let fake_runner_args = parse_fake_runner_args("subproject@branch_ref".to_string()).ok();
        add_fake_runner_env(&mut command, &fake_runner_args);

        // check output
        let output = command.output().await.unwrap();
        let env = String::from_utf8_lossy(&output.stdout);
        assert!(env.contains("GITHUB_REF_NAME=branch_ref"));
        assert!(env.contains("SUBPROJECT_DIR=subproject"));
    }

    #[tokio::test]
    async fn test_add_fake_runner_env_with_no_commit() {
        let mut command = Command::new("env");
        let fake_runner_args = parse_fake_runner_args("subproject".to_string()).ok();
        add_fake_runner_env(&mut command, &fake_runner_args);

        // check output
        let output = command.output().await.unwrap();
        let env = String::from_utf8_lossy(&output.stdout);
        assert!(!env.contains("GITHUB_REF_NAME"));
        assert!(env.contains("SUBPROJECT_DIR=subproject"));
    }

    #[tokio::test]
    async fn test_add_fake_runner_env_with_no_args() {
        let mut command = Command::new("env");
        add_fake_runner_env(&mut command, &None);

        // check output
        let output = command.output().await.unwrap();
        let env = String::from_utf8_lossy(&output.stdout);
        assert!(!env.contains("GITHUB_REF_NAME"));
        assert!(!env.contains("SUBPROJECT_DIR"));
    }
}
