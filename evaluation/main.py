import argparse
import logging
import os
import subprocess
import sys
import time
from datetime import datetime
from dotenv import load_dotenv

from src.runner import Runner, RunConfiguration, GitHubConfig
from src.scenario_parser import parse_scenario_csv

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s %(name)s [%(levelname)s] %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger(__name__)


def run_scenario(scenario_path: str, timeout_seconds: int, working_dir: str, output_dir: str | None):
    # Load environment variables from parent directory
    env_path = os.path.join(os.path.dirname(os.path.dirname(__file__)), '.env')
    load_dotenv(env_path)

    # Read GitHub configuration
    github_config = None
    github_repository = os.getenv('GITHUB_REPOSITORY')
    github_token = os.getenv('GITHUB_PAT_TOKEN')
    if github_repository and github_token:
        github_config = GitHubConfig(
            repository=github_repository,
            pat_token=github_token
        )
        logger.info(f"Loaded GitHub configuration for repository: {github_repository}")
    else:
        logger.warning("GitHub configuration not found in .env file")

    # scenario_path might be the CSV file path or the directory path
    if os.path.isdir(scenario_path):
        scenario_path = os.path.join(scenario_path, 'scenario.csv')

    output_dir_name = output_dir or f'output_{datetime.now().strftime("%Y-%m-%d_%H-%M-%S")}'
    scenario_base_path = os.path.dirname(scenario_path)
    output_dir = os.path.join(scenario_base_path, output_dir_name)
    os.makedirs(output_dir, exist_ok=True)
    logger.info(f'Output directory: {output_dir}')

    # Copy scenario csv file over for later
    scenario_output_path = os.path.join(output_dir, 'scenario.csv')
    subprocess.run(['cp', scenario_path, scenario_output_path])

    scenario = parse_scenario_csv(scenario_path)
    for idx, run in enumerate(scenario.runs):
        print("--------------------")
        print(f"Run: {run.name} ({idx + 1}/{len(scenario.runs)})")
        print("--------------------")

        run_configuration = RunConfiguration(
            run=run,
            timeout_seconds=timeout_seconds,
            working_dir=working_dir,
            github_config=github_config
        )
        runner = Runner(run_configuration)
        output = runner.execute()

        # Write the output logs to the output directory
        with open(os.path.join(output_dir, f'{run.name}.log'), 'w') as f:
            f.write(output.stdout)
        with open(os.path.join(output_dir, f'{run.name}.err'), 'w') as f:
            f.write(output.stderr)

        # Wait a few seconds to ensure enclaves are down (and everything just went back to idle)
        time.sleep(5)


def main():
    # Get the directory containing main.py
    current_dir = os.path.dirname(os.path.abspath(__file__))
    default_working_dir = os.path.dirname(current_dir)  # Parent directory

    parser = argparse.ArgumentParser(
        description='Run the full evaluation based on a scenario file')
    parser.add_argument(
        'scenario',
        type=str,
        help='Read a scenario from a CSV file'
    )
    parser.add_argument(
        '--timeout',
        type=int,
        default=120,
        help='Timeout for the host server in seconds'
    )
    parser.add_argument(
        '--working-dir',
        type=str,
        default=default_working_dir,
        help='Working directory containing the host-server binary (defaults to parent of evaluation folder)'
    )
    parser.add_argument(
        '--output-dir',
        type=str,
        default=None,
        help='Output directory to store the logs (defaults to a new folder named using timestamps)'
    )
    args = parser.parse_args()

    # Verify we are root, or otherwise get sudo active
    if os.geteuid() != 0:
        logger.info("Currently not root. Will verify sudo access")
        # Verify sudo access by running whoami and checking output is root
        try:
            result = subprocess.run(['sudo', 'whoami'], check=True, capture_output=True, text=True)
            if result.stdout.strip() != 'root':
                logger.error("Error: sudo did not elevate to root privileges")
                sys.exit(1)
        except subprocess.CalledProcessError:
            logger.error(
                "Error: This script requires sudo privileges. Please run with sudo.")
            sys.exit(1)

    run_scenario(args.scenario, args.timeout, args.working_dir, args.output_dir)


if __name__ == '__main__':
    main()
