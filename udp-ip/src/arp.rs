use crate::{
    address::{MacAddr, BROADCAST_MAC_ADDR},
    ethernet::{EthernetFrame, EthernetType, ETHERNET_FRAME_LENGTH},
    net::NetworkInterface,
    socket,
};
use std::net::Ipv4Addr;
use tracing::info;

pub struct Arp {}

impl Arp {
    pub fn send(dst_ip_addr: Ipv4Addr, src_net_interface: &NetworkInterface) -> Option<ArpFrame> {
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
        let mut send_packet = Vec::new();
        send_packet.extend(ethernet_frame.to_bytes());
        send_packet.extend(arp_req_frame.to_bytes());

        let (sender, mut reciever) = socket::channel(&src_net_interface);
        info!("send the arp packet...");
        sender.sendto(send_packet);

        // パケットの送受信
        info!("receive the arp packet...");
        while let Ok((_ret, _addr)) = reciever.recvfrom() {
            if !reciever.buf.is_empty() && Arp::is_arp_reply_packet(&reciever.buf) {
                info!("found an arp reply packet...");
                let offset = ETHERNET_FRAME_LENGTH;
                return Some(ArpFrame::from_bytes(&reciever.buf[offset..]));
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
    pub sender_mac_addr: MacAddr,
    pub sender_ip_addr: Ipv4Addr,
    pub target_mac_addr: MacAddr,
    pub target_ip_addr: Ipv4Addr,
}

impl ArpFrame {
    pub fn new_request(
        target_ip_addr: Ipv4Addr,
        sender_mac_addr: MacAddr,
        sender_ip_addr: Ipv4Addr,
    ) -> Self {
        ArpFrame {
            hardware_type: 0x0001, // Ethernet
            protocol_type: 0x0800, // IPv4
            hardware_size: 0x06,   // mac address は 6 bytes
            protocol_size: 0x04,   // IP address は 4 bytes
            opcode: 0x0001,        // ARP request
            sender_mac_addr,
            sender_ip_addr,
            target_mac_addr: BROADCAST_MAC_ADDR,
            target_ip_addr,
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
        bytes.extend(self.sender_mac_addr.octets());
        bytes.extend(self.sender_ip_addr.octets());
        bytes.extend(self.target_mac_addr.octets());
        bytes.extend(self.target_ip_addr.octets());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            hardware_type: u16::from_be_bytes([bytes[0], bytes[1]]),
            protocol_type: u16::from_be_bytes([bytes[2], bytes[3]]),
            hardware_size: bytes[4],
            protocol_size: bytes[5],
            opcode: u16::from_be_bytes([bytes[6], bytes[7]]),
            sender_mac_addr: MacAddr::new(
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            ),
            sender_ip_addr: Ipv4Addr::new(bytes[14], bytes[15], bytes[16], bytes[17]),
            target_mac_addr: MacAddr::new(
                bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            ),
            target_ip_addr: Ipv4Addr::new(bytes[24], bytes[25], bytes[26], bytes[27]),
        }
    }
}
