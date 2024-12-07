install:
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.81.0 -y
	. "$HOME/.cargo/env"
	sudo apt-get update -y && sudo apt-get install -y iputils-ping build-essential
	
setup-netns:
	./scripts/setup-netns.sh

arp-test:
	cargo build
	sudo ip netns exec host1 ./target/debug/toy-protocol arp host1-veth0 192.0.2.2

clean-netns:
	sudo ip --all netns delete
