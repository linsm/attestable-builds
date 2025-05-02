#!/bin/bash
set -e -x;

# Go to the directory where the script is located
cd "$(dirname "$0")";
sudo yum install -y $(cat setup-aws-install-packages.list);