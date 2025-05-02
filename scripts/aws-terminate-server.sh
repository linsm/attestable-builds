#!/bin/bash
set -e;
. .env;

# Read the tag as the first argument from the CLI
if [ -z "$1" ]; then
    echo "Usage: $0 <tag>";
    exit 1;
fi
TAG="$1";
echo "[ ] Tag: $TAG";

# Check if there is no instance with tag `$TAG`; in that case exit
# shellcheck disable=SC2046
if [ $(aws ec2 describe-instances --filters "Name=tag:LSID,Values=$TAG" --query 'Reservations[*].Instances[*].[InstanceId]' --output text | wc -l) -eq 0 ]; then
    echo "[ ] No instance with tag '$TAG' exists. Exiting."
    exit 0
fi

# Otherwise find the instance and terminate it
echo "[ ] Terminate instance with tag '$TAG'";
INSTANCE_ID=$(aws ec2 describe-instances --filters "Name=tag:LSID,Values=$TAG" --query 'Reservations[*].Instances[*].[InstanceId]' --output text);
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID" --no-paginate;
echo "[+] Instance terminated";

# Remove tag from instance
echo "[ ] Removing tag from instance";
aws ec2 delete-tags --resources "$INSTANCE_ID" --tags Key=LSID;
echo "[+] Tag removed from instance";

echo "[+] ALL DONE";
