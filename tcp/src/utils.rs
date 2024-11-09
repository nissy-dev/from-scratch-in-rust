pub type MacAddr = [u8; 6];

const BROADCAST_MAC_ADDR: MacAddr = [0xff; 6];

fn parse_mac_addr(addr: &str) -> MacAddr {
    let mut mac_addr = [0; 6];
    let mut i = 0;
    for byte in addr.split(':') {
        mac_addr[i] = u8::from_str_radix(byte, 16).unwrap();
        i += 1;
    }
    mac_addr
}

pub type IpAddr = [u8; 4];
