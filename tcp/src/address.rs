use core::str;
use std::fmt;

#[derive(Clone, Copy)]
pub struct MacAddr {
    octets: [u8; 6],
}

pub const BROADCAST_MAC_ADDR: MacAddr = MacAddr { octets: [0xff; 6] };

impl MacAddr {
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> MacAddr {
        MacAddr {
            octets: [a, b, c, d, e, f],
        }
    }

    pub fn octets(&self) -> [u8; 6] {
        self.octets
    }
}

impl From<&str> for MacAddr {
    // TODO: MacAddress はハイフンでつなげる場合もある
    fn from(value: &str) -> MacAddr {
        let mut mac_addr = [0; 6];
        for (i, byte) in value.split(':').enumerate() {
            mac_addr[i] = u8::from_str_radix(byte, 16).expect("invalid mac address");
        }
        MacAddr { octets: mac_addr }
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.octets[0],
            self.octets[1],
            self.octets[2],
            self.octets[3],
            self.octets[4],
            self.octets[5],
        ))
    }
}

impl fmt::Debug for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
