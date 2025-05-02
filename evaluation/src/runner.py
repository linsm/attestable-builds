import logging
import os
import re
import select
import subprocess
import time
from dataclasses import dataclass, field
from io import StringIO

from src.scenario_parser import ScenarioRun
from src.github import trigger_workflow

logger = logging.getLogger(__name__)


@dataclass
class GitHubConfig:
    repository: str
    pat_token: str


@dataclass
class RunConfiguration:
    run: ScenarioRun
    timeout_seconds: int
    working_dir: str
    github_config: GitHubConfig | None = None
    kill_regexes: list[str] = field(default_factory=lambda: [
        "Enclave client finished",
        "Finished interacting with the enclave client",
        "Host proxy failed to start",
    ])


@dataclass
class RunOutputs:
    error: str | None
    stdout: str
    stderr: str

    def success(self) -> bool:
        return self.error is None


class Runner:
    def __init__(self, run_configuration: RunConfiguration):
        self.run_configuration = run_configuration

    def execute(self) -> RunOutputs:
        # For real runners, trigger the GitHub workflow first
        if self.run_configuration.run.use_real_runner:
            if not self.run_configuration.github_config:
                return RunOutputs(
                    error="GitHub configuration not provided for real runner",
                    stdout="",
                    stderr="Missing GitHub configuration"
                )

            # The yaml file for the workflow is the same as the subproject directory
            workflow_id = self.run_configuration.run.target.subproject_dir
            if not trigger_workflow(
                repository=self.run_configuration.github_config.repository,
                branch=self.run_configuration.run.target.branch_ref or "main",
                workflow_id=workflow_id,
                github_token=self.run_configuration.github_config.pat_token
            ):
                return RunOutputs(
                    error="Failed to trigger GitHub workflow",
                    stdout="",
                    stderr="Failed to trigger workflow dispatch event"
                )
            logger.info(
                f"Successfully triggered workflow for {self.run_configuration.github_config.repository} "
                f"with workflow_id {workflow_id}"
            )

        # Start the host server according to the run configuration
        # - Ensure the run configuration arguments are passed to the host server
        # - Capture the logs from the host server
        # - Wait for the host server to finish
        # - If the timeout is reached, kill the host server and return the logs
        # - If the host server fails, return the logs and raise an error
        logger.info(f"Starting host server with configuration: {self.run_configuration}")

        # Prepare command based on run configuration
        cmd = [
            "sudo",  # Run with sudo privileges
            os.path.join(self.run_configuration.working_dir, "target/debug/host-server"),
            self.run_configuration.run.runner_start_mode.to_start_mode(),
            f"--runner-start-mode={self.run_configuration.run.runner_start_mode.to_runner_mode()}",
            "--simulate-log-publishing",  # We simulate the log publishing for testing
            "--simulate-webhook-event",  # We simulate the webhook for testing
        ]

        # Only add fake runner argument if we're not using a real runner
        if not self.run_configuration.run.use_real_runner:
            # Construct the fake runner argument
            fake_runner_arg = f"--simulate-client-use-fake-runner={self.run_configuration.run.target.subproject_dir}"
            if self.run_configuration.run.target.branch_ref:
                fake_runner_arg += f"@{self.run_configuration.run.target.branch_ref}"
            cmd.append(fake_runner_arg)

        if self.run_configuration.run.fake_attestation:
            cmd.append("--simulate-client-use-fake-attestation")

        if self.run_configuration.run.big_job:
            cmd.append("--big-job")

        logger.info(f"Executing command: {' '.join(cmd)}")

        try:
            # Start the process with output capture
            process = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1024,
                universal_newlines=True,
                cwd=self.run_configuration.working_dir
            )

            # Initialize output buffers
            stdout_buffer = StringIO()
            stderr_buffer = StringIO()

            # Wait for timeout or completion
            start_time = time.time()
            while process.poll() is None:
                print(".", end="", flush=True)
                if time.time() - start_time > self.run_configuration.timeout_seconds:
                    process.kill()
                    stdout, stderr = process.communicate()
                    return RunOutputs(
                        error="Host server timed out",
                        stdout=stdout_buffer.getvalue() + stdout,
                        stderr=stderr_buffer.getvalue() + stderr
                    )

                # Use select to handle both stdout and stderr without blocking
                rlist = []
                if process.stdout:
                    rlist.append(process.stdout)
                if process.stderr:
                    rlist.append(process.stderr)

                def kill_if_line_matches(candidate: str) -> bool:
                    for regex in self.run_configuration.kill_regexes:
                        if re.search(regex, candidate):
                            logger.info(f"Killing host server because of regex: '{regex}' in line: '{candidate}'")
                            return True
                    return False

                if rlist:
                    readable, _, _ = select.select(rlist, [], [], 0.1)
                    for stream in readable:
                        while True:
                            line = stream.readline()
                            if not line:
                                break
                            line = line.rstrip()
                            if stream == process.stdout:
                                stdout_buffer.write(line + "\n")
                                print(f"[host-server:stdout] {line}")
                            else:
                                stderr_buffer.write(line + "\n")
                                print(f"[host-server:STDERR] {line}")
                            if kill_if_line_matches(line):
                                # If real, wait a bit longer for the sake of the GitHub runner
                                if self.run_configuration.run.use_real_runner:
                                    time.sleep(8)
                                time.sleep(2)
                                process.kill()
                                stdout, stderr = process.communicate()
                                return RunOutputs(
                                    error="Host server killed by regex",
                                    stdout=stdout_buffer.getvalue() + stdout,
                                    stderr=stderr_buffer.getvalue() + stderr
                                )

                time.sleep(0.1)  # Small sleep to prevent CPU spinning

            stdout, stderr = process.communicate()
            for line in stderr.splitlines():
                if line:
                    stderr_buffer.write(line + "\n")
                    print(f"[host-server:STDERR] {line}")
            for line in stdout.splitlines():
                if line:
                    stdout_buffer.write(line + "\n")
                    print(f"[host-server:stdout] {line}")

            if process.returncode != 0:
                error_msg = f"Host server failed with return code {process.returncode}"
                logger.error(error_msg)
                return RunOutputs(
                    error=error_msg,
                    stdout=stdout_buffer.getvalue(),
                    stderr=stderr_buffer.getvalue()
                )

            return RunOutputs(
                error=None,
                stdout=stdout_buffer.getvalue(),
                stderr=stderr_buffer.getvalue()
            )

        except Exception as e:
            error_msg = f"Failed to execute host server: {str(e)}"
            logger.error(error_msg)
            return RunOutputs(
                error=error_msg,
                stdout="",
                stderr=str(e)
            )
