# toy-tcp

- https://zenn.dev/satoken/articles/golang-tcpip

## 環境構築

```sh
# ローカル PC での設定
## 仮想環境の作成
multipass launch 22.04 --cpus 2 --disk 25GiB --name toy-tcp --mount $(pwd):/home/ubuntu/toy-tcp
## 仮想環境に入る
multipass shell toy-tcp

# 仮想環境内での設定
ubuntu@toy-tcp:~$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.81.0 -y
ubuntu@toy-tcp:~$ . "$HOME/.cargo/env"
ubuntu@toy-tcp:~$ sudo apt install -y build-essential
```

### VSCode での開発

[Multipassで作成した仮想マシンにsshで接続する](https://note.com/inagy/n/ndee177461a6e)を参考に、仮想環境に ssh して開発する。

## 仮想ネットワークの作成

[Network Namespace を使用してネットワークに触れてみる](https://qiita.com/babashoya/items/7eb7bd69538730e43705) を参考に作成する。
