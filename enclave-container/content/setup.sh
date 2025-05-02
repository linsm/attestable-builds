#!/bin/bash
ulimit -n 65536

# sudo complains if the hostname is missing or canont be resolved
echo "enclave" > /proc/sys/kernel/hostname
echo "127.0.0.1 enclave" >> /etc/hosts

echo "TIMESTAMP ENCLAVE_NET_SETUP_START `date -Ins`"

# setting an address for loopback
ifconfig lo 127.0.0.1
ifconfig

# adding a default route
ip route add default via 127.0.0.1 dev lo
route -n

# required for ubuntu which uses nftable
update-alternatives --set iptables /usr/sbin/iptables-legacy

# iptables rules to route traffic to transparent proxy
iptables -A OUTPUT -t nat -p tcp --dport 1:65535 ! -d 127.0.0.1  -j DNAT --to-destination 127.0.0.1:1200
iptables -t nat -A POSTROUTING -o lo -s 0.0.0.0 -j SNAT --to-source 127.0.0.1
iptables -L -t nat

# setup network namespace for sandbox in enclave
ip netns add netns-sandbox
ip link add veth0 type veth peer name seth0
ip link set veth0 up
ip link set seth0 netns netns-sandbox
ip addr add 172.16.0.1/16 dev veth0
echo 1 > /proc/sys/net/ipv4/conf/veth0/route_localnet
echo 1 > /proc/sys/net/ipv4/ip_forward
nsenter --net=/run/netns/netns-sandbox ip link set veth0 up
nsenter --net=/run/netns/netns-sandbox ip link set seth0 up
nsenter --net=/run/netns/netns-sandbox ip addr add 172.16.0.2/16 dev seth0
nsenter --net=/run/netns/netns-sandbox ip route add default via 172.16.0.1
nsenter --net=/run/netns/netns-sandbox iptables -t nat -A OUTPUT -p udp --dport 53 -j DNAT --to-destination 172.16.0.1:53
iptables -t nat -A PREROUTING -i veth0 -p tcp --dport 1:65535 -j DNAT --to-destination 127.0.0.1:1200
iptables -A FORWARD -i veth0 -o lo -p tcp --dport 1:65535 -j ACCEPT

echo "TIMESTAMP ENCLAVE_NET_SETUP_END `date -Ins`"

echo "TIMESTAMP ENCLAVE_EXTRACT_ROOTFS_START `date -Ins`"

# setup rootfs directory
mkdir -p /app/sandbox-container/build/rootfs
tar --zstd -xf /app/sandbox-container/build/rootfs.tar.zst -C /app/sandbox-container/build/rootfs
rm /app/sandbox-container/build/rootfs.tar.zst

# set resolv.conf of sandbox to use enclave for DNS
echo "nameserver 172.16.0.1" > /app/sandbox-container/build/rootfs/etc/resolv.conf

echo "TIMESTAMP ENCLAVE_EXTRACT_ROOTFS_END `date -Ins`"

echo "TIMESTAMP ENCLAVE_NET_SERVICES_START `date -Ins`"

# setup dns proxy
/app/dnsproxy -u https://1.1.1.1/dns-query&
/app/ip-to-vsock-transparent --ip-addr 127.0.0.1:1200 --vsock-addr 3:5000&

# enable ssh and listen for incoming connections
/etc/init.d/ssh restart
socat VSOCK-LISTEN:22,reuseaddr,fork TCP:127.0.0.1:22&

echo "TIMESTAMP ENCLAVE_NET_SERVICES_END `date -Ins`"
