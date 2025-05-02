#!/bin/bash
sudo killall make
sudo killall time
sudo killall python
sudo killall python3
sudo killall host-server
sudo killall vsock-to-ip-transparent
sudo killall node # VS code
sudo nitro-cli terminate-enclave --all
