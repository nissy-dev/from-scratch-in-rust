#!/usr/bin/env bash
# -u: Fail on when existing unset variables
# -e -o pipefail: Fail on when happening command errors
set -ueo pipefail

# 名前空間の作成
sudo ip netns add host1
sudo ip netns add router
sudo ip netns add host2

# veth の作成とネットワークインターフェイスの割り当て
sudo ip link add host1-veth0 type veth peer name gw-veth0
sudo ip link add host2-veth0 type veth peer name gw-veth1
sudo ip link set host1-veth0 netns host1
sudo ip link set gw-veth0 netns router
sudo ip link set gw-veth1 netns router
sudo ip link set host2-veth0 netns host2

# IP アドレスをネットワークインターフェイスへ設定
sudo ip netns exec host1 ip address add 192.0.2.1/24 dev host1-veth0
sudo ip netns exec router ip address add 192.0.2.254/24 dev gw-veth0
sudo ip netns exec router ip address add 198.51.100.254/24 dev gw-veth1
sudo ip netns exec host2 ip address add 198.51.100.1/24 dev host2-veth0

# ネットワークインターフェイスの起動
sudo ip netns exec host1 ip link set host1-veth0 up
sudo ip netns exec router ip link set gw-veth0 up
sudo ip netns exec router ip link set gw-veth1 up
sudo ip netns exec host2 ip link set host2-veth0 up

# ルーティングの設定
sudo ip netns exec host1 ip route add default via 192.0.2.254
sudo ip netns exec host2 ip route add default via 198.51.100.254

#  IPv4 のルータとして動作するように設定を変更
sudo ip netns exec router sysctl net.ipv4.ip_forward=1
