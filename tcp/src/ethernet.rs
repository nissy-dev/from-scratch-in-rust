use crate::utils::MacAddr;

pub enum EthernetType {
    Ipv4,
    Arp,
}

const IPV4: [u8; 2] = [0x08, 0x00];
const ARP: [u8; 2] = [0x08, 0x06];

struct EthernetFrame {
    frame_type: [u8; 2],
    dst_mac_addr: MacAddr,
    src_mac_addr: MacAddr,
}

impl EthernetFrame {
    fn new(dst_mac_addr: MacAddr, src_mac_addr: MacAddr, ethernet_type: EthernetType) -> Self {
        EthernetFrame {
            frame_type: match ethernet_type {
                EthernetType::Ipv4 => IPV4,
                EthernetType::Arp => ARP,
            },
            dst_mac_addr,
            src_mac_addr,
        }
    }
}
