#!/bin/bash
set -e
ip netns delete netns-sandbox || true;
sleep 1

echo "[ ] Reading local network interface from .env"
source .env
echo "LOCAL_NETWORK_INTERFACE=$LOCAL_NETWORK_INTERFACE"

set -x

echo "[ ] Setting up network namespace for sandbox in enclave"
# veth0 is the host side of the veth pair
# seth0 is the sandbox side of the veth pair
ip netns add netns-sandbox;
ip link add veth0 type veth peer name seth0;

echo "[ ] Setting up the host side of the veth pair"
# Start the host side
ip link set veth0 up;
ip addr add 192.168.0.2/24 dev veth0;

echo "[ ] Setting up the sandbox side of the veth pair"
# Move the sandbox side of the veth pair into the namespace
ip link set seth0 netns netns-sandbox;

# Give it an ip and start it up
ip netns exec netns-sandbox ip addr add 192.168.0.1/24 dev seth0;
ip netns exec netns-sandbox ip link set seth0 up;
ip netns exec netns-sandbox ip route add default via 192.168.0.2 dev seth0;

echo "[ ] Configuring the host for NAT and routing"
echo 1 > /proc/sys/net/ipv4/conf/veth0/route_localnet;
echo 1 > /proc/sys/net/ipv4/ip_forward;
iptables -t nat -A POSTROUTING -s 192.168.0.2/24 -o "$LOCAL_NETWORK_INTERFACE" -j MASQUERADE

ip netns exec netns-sandbox curl 1.1.1.1
echo "[+] All done (if you see any html above)"
