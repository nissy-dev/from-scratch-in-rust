use crate::{
    address::{MacAddr, BROADCAST_MAC_ADDR},
    ethernet::{EthernetFrame, EthernetType},
    net::NetworkInterface,
};
use nix::sys::socket::{
    bind, recvfrom, sendto, socket, AddressFamily, LinkAddr, MsgFlags, SockFlag, SockProtocol,
    SockType, SockaddrLike, SockaddrStorage,
};
use nix::unistd::close;
use std::{net::Ipv4Addr, os::fd::AsRawFd};
use tracing::debug;

pub struct Arp {}

impl Arp {
    pub fn send(dst_ip_addr: Ipv4Addr, src_net_interface: NetworkInterface) -> Option<ArpFrame> {
        // socket の作成
        debug!("create raw socket....");
        let sock_fd = socket(
            AddressFamily::Packet,
            SockType::Raw,
            SockFlag::empty(),
            SockProtocol::EthAll,
        )
        .expect("failed to create a socket");

        // socket にアドレスを紐づけて、受け取るパケットを制限する
        // これをしないと全てのアドレスからパケットを受け取ってしまう
        // https://man7.org/linux/man-pages/man7/packet.7.html
        let sockaddr = &nix::libc::sockaddr_ll {
            sll_family: nix::libc::AF_PACKET as u16,
            sll_protocol: nix::libc::ETH_P_ARP as u16,
            sll_ifindex: src_net_interface.ifindex(),
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: src_net_interface.sll_halen(),
            sll_addr: src_net_interface.sll_addr(),
        };
        let sock_addr = unsafe {
            LinkAddr::from_raw(
                sockaddr as *const nix::libc::sockaddr_ll as *const nix::libc::sockaddr,
                None,
            )
            .expect("failed to create link address")
        };
        debug!("bind sll_address to the socket....");
        bind(sock_fd.as_raw_fd(), &sock_addr).expect("failed to bind sll_address to packet");

        // 送信するパケットの準備
        let ethernet_frame = EthernetFrame::new(
            EthernetType::Arp,
            BROADCAST_MAC_ADDR,
            src_net_interface.mac_addr,
        );
        let arp_req_frame = ArpFrame::new_request(
            dst_ip_addr,
            src_net_interface.mac_addr,
            src_net_interface.ip_addr,
        );
        let mut packet = Vec::new();
        packet.extend(ethernet_frame.to_bytes());
        packet.extend(arp_req_frame.to_bytes());

        // パケットの送信
        debug!("send the socket....");
        sendto(sock_fd.as_raw_fd(), &packet, &sock_addr, MsgFlags::empty())
            .expect("failed to send the arp socket");

        // パケットの受信
        debug!("receive the socket....");
        let mut recv_buf = vec![0; 4096];
        while let Ok((_ret, _addr)) =
            recvfrom::<SockaddrStorage>(sock_fd.as_raw_fd(), &mut recv_buf)
        {
            if !recv_buf.is_empty() && Arp::is_arp_reply_packet(&recv_buf) {
                debug!("found arp reply packet....");
                close(sock_fd.as_raw_fd()).expect("failed to close a socket connection");
                return Some(ArpFrame::from_bytes(&recv_buf[14..]));
            }
        }

        None
    }

    fn is_arp_reply_packet(packet: &[u8]) -> bool {
        packet[12] == 0x08 && packet[13] == 0x06 && packet[20] == 0x00 && packet[21] == 0x02
    }
}

#[derive(Debug)]
pub struct ArpFrame {
    // hardware type, protocol type, opcode は下記のリンクが参考になる
    // https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml
    pub hardware_type: u16,
    pub hardware_size: u8,
    pub protocol_type: u16,
    pub protocol_size: u8,
    pub opcode: u16,
    pub src_mac_addr: MacAddr,
    pub src_ip_addr: Ipv4Addr,
    pub dst_mac_addr: MacAddr,
    pub dst_ip_addr: Ipv4Addr,
}

impl ArpFrame {
    pub fn new_request(
        dst_ip_addr: Ipv4Addr,
        src_mac_addr: MacAddr,
        src_ip_addr: Ipv4Addr,
    ) -> Self {
        ArpFrame {
            hardware_type: 0x0001, // ethernet
            protocol_type: 0x0800, // IPv4
            hardware_size: 0x06,
            protocol_size: 0x04,
            opcode: 0x0001, // ARP request
            src_mac_addr,
            src_ip_addr,
            dst_mac_addr: BROADCAST_MAC_ADDR,
            dst_ip_addr,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // u16 から u8 への変換には、little edian と big edian の２つの方法がある
        // Network byte order では big edian が利用される
        bytes.extend(self.hardware_type.to_be_bytes());
        bytes.extend(self.protocol_type.to_be_bytes());
        bytes.push(self.hardware_size);
        bytes.push(self.protocol_size);
        bytes.extend(self.opcode.to_be_bytes());
        bytes.extend(self.src_mac_addr.octets());
        bytes.extend(self.src_ip_addr.octets());
        bytes.extend(self.dst_mac_addr.octets());
        bytes.extend(self.dst_mac_addr.octets());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            hardware_type: u16::from_be_bytes([bytes[0], bytes[1]]),
            protocol_type: u16::from_be_bytes([bytes[2], bytes[3]]),
            hardware_size: bytes[4],
            protocol_size: bytes[5],
            opcode: u16::from_be_bytes([bytes[6], bytes[7]]),
            src_mac_addr: MacAddr::new(
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            ),
            src_ip_addr: Ipv4Addr::new(bytes[14], bytes[15], bytes[16], bytes[17]),
            dst_mac_addr: MacAddr::new(
                bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            ),
            dst_ip_addr: Ipv4Addr::new(bytes[24], bytes[25], bytes[26], bytes[27]),
        }
    }
}
