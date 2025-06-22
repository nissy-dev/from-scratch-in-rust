use tracing::info;

use crate::{
    address::MacAddr,
    checksum,
    ethernet::{EthernetFrame, EthernetType},
    ip::{IpHeader, Protocol, IP_HEADER_LENGTH},
    net::NetworkInterface,
    socket,
};
use std::net::Ipv4Addr;

const UDP_HEADER_LENGTH: usize = 8;

pub struct Udp {}

impl Udp {
    pub fn send(
        dst_port: u16,
        dst_ip_addr: Ipv4Addr,
        dst_mac_addr: MacAddr,
        src_net_interface: &NetworkInterface,
    ) {
        let payload = b"Hello, UDP!";
        let ip_packet_length = (IP_HEADER_LENGTH + UDP_HEADER_LENGTH + payload.len()) as u16;
        let ethernet_frame =
            EthernetFrame::new(EthernetType::Ipv4, dst_mac_addr, src_net_interface.mac_addr);
        let ip_header = IpHeader::new(
            src_net_interface.ip_addr,
            dst_ip_addr,
            ip_packet_length,
            Protocol::UDP,
        );
        let udp_packet_length = (UDP_HEADER_LENGTH + payload.len()) as u16;
        // src_port は適当に設定
        let udp_header = UdpHeader::new(23456, dst_port, udp_packet_length);
        let send_packet = [
            ethernet_frame.to_bytes(),
            ip_header.to_bytes(),
            udp_header.to_bytes(&ip_header, payload),
            payload.to_vec(),
        ]
        .concat();
        let (sender, _) = socket::channel(&src_net_interface);
        info!("send the UDP packet...");
        sender.sendto(send_packet);
    }
}

// cf: https://datatracker.ietf.org/doc/html/rfc768
pub struct UdpHeader {
    src_port: u16,
    dst_port: u16,
    length: u16,
    checksum: u16,
}

impl UdpHeader {
    pub fn new(src_port: u16, dst_port: u16, length: u16) -> UdpHeader {
        UdpHeader {
            src_port,
            dst_port,
            length,
            checksum: 0, // 後でセットする
        }
    }

    pub fn to_bytes(&self, ip_header: &IpHeader, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.src_port.to_be_bytes());
        bytes.extend(&self.dst_port.to_be_bytes());
        bytes.extend(&self.length.to_be_bytes());
        bytes.extend(&self.checksum.to_be_bytes());
        self.set_checksum(&mut bytes, ip_header, data);
        bytes
    }

    fn set_checksum(&self, bytes: &mut [u8], ip_header: &IpHeader, data: &[u8]) {
        let mut pseudo_header = [0; 12];
        pseudo_header[0..4].copy_from_slice(&ip_header.src_ip.octets());
        pseudo_header[4..8].copy_from_slice(&ip_header.dst_ip.octets());
        pseudo_header[8..10].copy_from_slice(&[0, 17]); // UDP protocol number
        let length = (bytes.len() + data.len()) as u16;
        pseudo_header[10..12].copy_from_slice(&length.to_be_bytes());

        let mut tmp_buf = [pseudo_header.to_vec(), bytes.to_vec(), data.to_vec()].concat();
        if tmp_buf.len() % 2 != 0 {
            tmp_buf.push(0);
        }

        let checksum = checksum::checksum(&tmp_buf);
        bytes[6..8].copy_from_slice(&checksum.to_be_bytes());
    }
}
