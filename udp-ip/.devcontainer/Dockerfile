FROM ubuntu:latest

RUN apt update && \
  apt upgrade -y && \
  apt install iproute2 -y && \
  apt install curl -y && \
  apt install git -y && \
  apt install make -y && \
  apt install tcpdump -y && \
  apt install iputils-ping -y && \
  apt install build-essential -y

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.81.0 -y
