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

# Discover the instance ID
INSTANCE_ID=$(aws ec2 describe-instances --filters "Name=tag:LSID,Values=$TAG" --query 'Reservations[*].Instances[*].[InstanceId]' --output text);
echo "[ ] Instance ID: $INSTANCE_ID";

# Increase the throughput of all attached volumnes to 1000 MB/s
echo "[ ] Increasing throughput of all attached volumes to 1000 MB/s";
for VOLUME_ID in $(aws ec2 describe-instances --instance-ids "$INSTANCE_ID" --query 'Reservations[*].Instances[*].BlockDeviceMappings[*].Ebs.VolumeId' --output text); do
    echo "[ ] Increasing throughput of volume $VOLUME_ID";
    aws ec2 modify-volume --volume-id "$VOLUME_ID" --throughput 1000 --iops 10000 --no-paginate;
    echo "[+] Throughput of volume $VOLUME_ID increased to 1000 MB/s. This takes a few minutes to apply.";
done

echo "[+] ALL DONE";
