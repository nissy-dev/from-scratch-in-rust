# TCP/IP
tuntap:
	sudo ip tuntap add mode tun dev tun0 && \
	sudo ip link set tun0 up && \
	sudo ip addr add 10.0.0.1/24 dev tun0

curl:
	curl --interface tun0 http://10.0.0.2/

# Wireshark
capture:
	tcpdump -i tun0 -w wireshark/capture.pcap
