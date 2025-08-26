#!/bin/bash

# Check if we are a big machine. We are a big machine if `nproc` is larger than 8
if [ $(nproc) -gt 8 ]; then
    echo "[ ] We are a big machine";
    TYPE="big";
else
    echo "[ ] We are a small machine";
    TYPE="regular";
fi

# If we are small...
if [ "$TYPE" == "regular" ]; then
cat > /etc/nitro_enclaves/allocator.yaml<< EOF
---
# Enclave configuration file.
#
# How much memory to allocate for enclaves (in MiB).
memory_mib: 4096
#
# How many CPUs to reserve for enclaves.
cpu_count: 2
EOF
else
cat > /etc/nitro_enclaves/allocator.yaml<< EOF
---
# Enclave configuration file.
#
# How much memory to allocate for enclaves (in MiB).
memory_mib: 62000
#
# How many CPUs to reserve for enclaves.
cpu_count: 16
EOF
fi
