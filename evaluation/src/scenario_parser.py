import csv
from dataclasses import dataclass
from enum import Enum


class RunnerMode(Enum):
    LOCAL_DIRECT = "local_direct"
    LOCAL_SANDBOX = "local_sandbox"
    ENCLAVE_DIRECT = "enclave_direct"
    ENCLAVE_SANDBOX = "enclave_sandbox"
    ENCLAVE_SANDBOX_PLUS = "enclave_sandbox_plus"

    def to_start_mode(self):
        return {
            RunnerMode.LOCAL_DIRECT: 'local',
            RunnerMode.LOCAL_SANDBOX: 'local',
            RunnerMode.ENCLAVE_DIRECT: 'nitro',
            RunnerMode.ENCLAVE_SANDBOX: 'nitro',
            RunnerMode.ENCLAVE_SANDBOX_PLUS: 'nitro'
        }[self]

    def to_runner_mode(self):
        return {
            RunnerMode.LOCAL_DIRECT: 'direct',
            RunnerMode.LOCAL_SANDBOX: 'sandbox',
            RunnerMode.ENCLAVE_DIRECT: 'direct',
            RunnerMode.ENCLAVE_SANDBOX: 'sandbox',
            RunnerMode.ENCLAVE_SANDBOX_PLUS: 'sandbox_plus'
        }[self]


@dataclass
class BuildTarget:
    subproject_dir: str
    branch_ref: str | None


@dataclass
class ScenarioRun:
    name: str
    runner_start_mode: RunnerMode
    fake_attestation: bool
    big_job: bool
    use_real_runner: bool
    target: BuildTarget


@dataclass
class Scenario:
    runs: list[ScenarioRun]


def parse_scenario_csv(csv_path: str) -> Scenario:
    runs = []
    with open(csv_path, 'r') as f:
        # Allow comments in the CSV file
        reader = csv.DictReader(filter(lambda line: line[0] != '#', f))
        for row in reader:
            name = row['name']
            mode = RunnerMode(row['runner_start_mode'].lower())

            fake_attestation = row['fake_attestation'].lower() == 'true'
            simulate_publishing = row['big_job'].lower() == 'true'
            use_real_runner = row['use_real_runner'].lower() == 'true'

            target_parts = row['target'].split('@')
            subproject_dir = target_parts[0]
            branch_ref = target_parts[1] if len(target_parts) > 1 else None
            target = BuildTarget(
                subproject_dir=subproject_dir,
                branch_ref=branch_ref
            )

            run = ScenarioRun(
                name=name,
                runner_start_mode=mode,
                fake_attestation=fake_attestation,
                big_job=simulate_publishing,
                use_real_runner=use_real_runner,
                target=target
            )
            runs.append(run)

    return Scenario(runs=runs)
