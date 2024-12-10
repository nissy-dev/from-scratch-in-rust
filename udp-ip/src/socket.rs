use nix::sys::socket::{
    bind, recvfrom, sendto, socket, AddressFamily, LinkAddr, MsgFlags, SockFlag, SockProtocol,
    SockType, SockaddrLike, SockaddrStorage,
};
use std::{
    os::fd::{AsRawFd, OwnedFd},
    sync::Arc,
};
use tracing::info;

use crate::net::NetworkInterface;

pub struct Sender {
    fd: Arc<OwnedFd>,
    addr: LinkAddr,
}

impl Sender {
    pub fn sendto(&self, packet: Vec<u8>) -> usize {
        sendto(self.fd.as_raw_fd(), &packet, &self.addr, MsgFlags::empty())
            .expect("failed to send the packet")
    }
}

pub struct Receiver {
    fd: Arc<OwnedFd>,
    pub buf: Vec<u8>,
}

impl Receiver {
    pub fn recvfrom(&mut self) -> nix::Result<(usize, Option<SockaddrStorage>)> {
        recvfrom::<SockaddrStorage>(self.fd.as_raw_fd(), &mut self.buf)
    }
}

pub fn channel(src_net_interface: &NetworkInterface) -> (Sender, Receiver) {
    // socket の作成
    info!("create raw socket....");
    let sock_fd = socket(
        AddressFamily::Packet,
        SockType::Raw,
        SockFlag::empty(),
        SockProtocol::EthAll,
    )
    .expect("failed to create a socket");

    // socket にアドレスを紐づけて、受け取るパケットを制限する
    // これをしないと全てのアドレスからパケットを受け取ってしまう
    // cf: https://man7.org/linux/man-pages/man7/packet.7.html
    let sock_addr = &nix::libc::sockaddr_ll {
        sll_family: nix::libc::AF_PACKET as u16,
        // ETH_P_ALL=0x0806 なので、big edian に変換しないと packet を receive できなかった
        // cf: https://thomask.sdf.org/blog/2017/09/01/layer-2-raw-sockets-on-rustlinux.html
        sll_protocol: (nix::libc::ETH_P_ALL as u16).to_be(),
        sll_ifindex: src_net_interface.ifindex(),
        sll_hatype: 0,
        sll_pkttype: 0,
        sll_halen: src_net_interface.sll_halen(),
        sll_addr: src_net_interface.sll_addr(),
    };
    let addr = unsafe {
        LinkAddr::from_raw(
            sock_addr as *const nix::libc::sockaddr_ll as *const nix::libc::sockaddr,
            None,
        )
        .expect("failed to create link address")
    };
    info!("bind sll_address to the socket....");
    bind(sock_fd.as_raw_fd(), &addr).expect("failed to bind sll_address to packet");

    let fd = Arc::new(sock_fd);
    let sender = Sender {
        fd: fd.clone(),
        addr,
    };
    let receiver = Receiver {
        fd,
        buf: vec![0; 4096],
    };
    (sender, receiver)
}
