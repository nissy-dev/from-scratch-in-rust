setup-netns:
	./scripts/setup-netns.sh

arp-test:
	cargo build
	ip netns exec host1 ./target/debug/toy-protocol arp host1-veth0 192.0.2.2

icmp-test:
	cargo build
	ip netns exec host1 ./target/debug/toy-protocol icmp host1-veth0 192.0.2.2

udp-test:
	cargo build
	ip netns exec host1 ./target/debug/toy-protocol udp host1-veth0 192.0.2.2 12345

wait-udp:
	ip netns exec host2 nc -ul 12345

clean-netns:
	ip --all netns delete

capture:
	ip netns exec host1 tcpdump -i host1-veth0 -w wireshark/capture.pcap
