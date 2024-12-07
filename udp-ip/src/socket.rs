use nix::sys::socket::{
    bind, recvfrom, sendto, socket, AddressFamily, LinkAddr, MsgFlags, SockFlag, SockProtocol,
    SockType, SockaddrLike, SockaddrStorage,
};
use std::os::fd::AsRawFd;
use tracing::info;

use crate::net::NetworkInterface;

pub fn send_and_recv(
    src_net_interface: &NetworkInterface,
    send_packet: &[u8],
    recv_packet: &mut [u8],
) -> nix::Result<(usize, Option<SockaddrStorage>)> {
    // socket の作成
    info!("create raw socket....");
    let socket = socket(
        AddressFamily::Packet,
        SockType::Raw,
        SockFlag::empty(),
        SockProtocol::EthAll,
    )
    .expect("failed to create a socket");
    let sock_fd = socket.as_raw_fd();

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
    bind(sock_fd, &addr).expect("failed to bind sll_address to packet");

    info!("send the packet...");
    sendto(sock_fd, &send_packet, &addr, MsgFlags::empty()).expect("faile to send the packet");

    info!("received the packet....");
    recvfrom::<SockaddrStorage>(sock_fd, recv_packet)
}
