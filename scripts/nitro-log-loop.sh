#!/bin/bash
nitro-cli console --enclave-name enclave;
while [ $? -ne 0 ]; do
    echo "----------------------------------";
    date;
    echo "----------------------------------";

    sleep 2; 

    nitro-cli console --enclave-name enclave || nitro-cli console --enclave-name enclave-wet;
done
