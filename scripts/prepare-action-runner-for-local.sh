#!/bin/bash
set -e;

echo "[ ] Reading information from .env file"
source .env
echo "[+] RUNNER_VERSION=${RUNNER_VERSION}"
echo "[+] RUNNER_USER=${RUNNER_USER}"

# echo "[ ] Install dependencies (Ubuntu only)"
# sudo apt-get update
# sudo apt-get install -y jq build-essential ca-certificates libssl-dev libffi-dev libicu74
# echo "[+] Dependencies installed"

echo "[ ] Download and install GitHub Actions Runner v${RUNNER_VERSION}"
sudo rm -rf "github-runner/${RUNNER_VERSION}" || true
mkdir -p "github-runner/${RUNNER_VERSION}"
cd "github-runner/${RUNNER_VERSION}"

curl -O -L https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
echo "[+] Downloaded the .tar.gz file"

tar xzf ./actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
rm -f ./actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz
echo "[+] Extracted everything"

echo "[ ] Chowing everything to the runner (and leave editable to the local user)"
cd ..
sudo chown -R ${RUNNER_USER}:${RUNNER_USER} ${RUNNER_VERSION}
sudo chown -R ${RUNNER_USER}:${RUNNER_USER} hooks
sudo chown -R ${RUNNER_USER}:${RUNNER_USER} output
sudo chown -R ${RUNNER_USER}:${RUNNER_USER} simulated
sudo chmod 777 -R hooks
sudo chmod 777 -R output
sudo chmod 777 -R simulated
echo "[+] Chown-ed everything"

echo "[!] Ensure that the 'stamps' folder is accessible for runner"
echo "    E.g. if it is in your home folder run 'chmod o+rx ~' but beware the consequences"
echo "    Alternatively, consider moving this project outside your home folder."
