# toy-tcp

- https://zenn.dev/satoken/articles/golang-tcpip

## 環境構築

github の codespace で開発した。必要なツール群は以下でインストール。

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.81.0 -y
. "$HOME/.cargo/env"
sudo apt-get update -y && sudo apt-get install -y iputils-ping build-essential
```

