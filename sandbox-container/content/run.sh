#!/bin/bash
set -e;

id;
env;

# Start runner config
cd "$GITHUB_RUNNER_PATH";
pwd;
ls -la .;

rm -f ".runner" ".credentials" ".credentials_rsaparams" "svc.sh" || true;

./config.sh --url "https://github.com/$GITHUB_REPOSITORY" --token "$GITHUB_REG_TOKEN" --ephemeral --disableupdate --unattended --replace --name "$GITHUB_RUNNER_NAME";
echo "RUNNER_CONFIGURATION_DONE=true" >> /app/github-runner/output/output.log;

# Then start the runner
./run.sh;
echo "RUNNER_FINISHED=true" >> /app/github-runner/output/output.log;
