use crate::address::MacAddr;

pub const ETHERNET_FRAME_LENGTH: usize = 14;

pub enum EthernetType {
    Ipv4,
    Arp,
}

const IPV4: [u8; 2] = [0x08, 0x00];
const ARP: [u8; 2] = [0x08, 0x06];

pub struct EthernetFrame {
    dst_mac_addr: MacAddr,
    src_mac_addr: MacAddr,
    frame_type: [u8; 2],
}

impl EthernetFrame {
    pub fn new(ethernet_type: EthernetType, dst_mac_addr: MacAddr, src_mac_addr: MacAddr) -> Self {
        EthernetFrame {
            dst_mac_addr,
            src_mac_addr,
            frame_type: match ethernet_type {
                EthernetType::Ipv4 => IPV4,
                EthernetType::Arp => ARP,
            },
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.dst_mac_addr.octets());
        bytes.extend(self.src_mac_addr.octets());
        bytes.extend(self.frame_type);
        bytes
    }
}
