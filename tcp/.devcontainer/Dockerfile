FROM ubuntu:latest
RUN apt-get update && apt-get install -y \
    sudo \
    curl \
    iproute2 \
    iptables \
    ethtool \
    net-tools \
    iputils-ping \
    netcat-openbsd \
    tcpdump \
    tshark \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="$HOME/.cargo/bin:${PATH}"
