enum EthernetFrameType {
    IPv4 = 0x0800,
    ARP = 0x0806,
}

struct EthernetFrame {
    dst_mac_addr: [u8; 6],
    src_mac_addr: [u8; 6],
    eth_type: EthernetFrameType,
}

impl EthernetFrame {
    fn new(dst_mac_addr: [u8; 6], src_mac_addr: [u8; 6], eth_type: EthernetFrameType) -> Self {
        EthernetFrame {
            dst_mac_addr,
            src_mac_addr,
            eth_type,
        }
    }
}
