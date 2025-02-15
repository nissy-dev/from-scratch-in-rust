#!/usr/bin/env bash
# -u: Fail on when existing unset variables
# -e -o pipefail: Fail on when happening command errors
set -ueo pipefail

# 名前空間の作成
ip netns add host1
ip netns add host2

# 仮想イーサネットピア (Virtual Ethernet, veth) の作成
# host1-veth0 と host2-veth0 はそれぞれ仮想ネットワークインターフェイスであり、
# それぞれがネットワークケーブルで接続されたペアとして振る舞う
ip link add host1-veth0 type veth peer name host2-veth0
# host1 に host1-veth0 のネットワークインターフェイスを割り当てる
ip link set host1-veth0 netns host1
ip link set host2-veth0 netns host2

# host1 でコマンドを実行する
# host1-veth0 に IP アドレスを割り当てる
ip netns exec host1 ip address add 192.0.2.1/24 dev host1-veth0
# ネットワークインターフェイスを立ち上げる
ip netns exec host1 ip link set host1-veth0 up

ip netns exec host2 ip address add 192.0.2.2/24 dev host2-veth0
ip netns exec host2 ip link set host2-veth0 up
