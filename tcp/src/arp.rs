use std::{
    net::Ipv4Addr,
    os::fd::{OwnedFd, RawFd},
};

use nix::libc::socket;

use crate::address::{MacAddr, BROADCAST_MAC_ADDR};

pub struct Arp {
    socket_fd: Option<OwnedFd>,
}

impl Arp {
    // fn new() -> Self {
    //     let fd = socket().expect("failed to create a socket for arp conection");
    //     Arp { socket_fd: fd }
    // }
    fn send(&self, fd: RawFd, ifindex: i32) {}
}

#[derive(Debug)]
struct ArpFrame {
    // hardware type, protocol type, opcode は下記のリンクが参考になる
    // https://www.iana.org/assignments/arp-parameters/arp-parameters.xhtml#arp-parameters-2
    pub hardware_type: u16,
    pub hardware_size: u8,
    pub protocol_type: u16,
    pub protocol_size: u8,
    pub opcode: u16,
    pub sender_mac: MacAddr,
    pub sender_ip: Ipv4Addr,
    pub target_mac: MacAddr,
    pub target_ip: Ipv4Addr,
}

impl ArpFrame {
    pub fn new_request(&self, sender_mac: &str, sender_ip: &str, target_ip: &str) -> Self {
        ArpFrame {
            hardware_type: 0x0001, // ethernet
            protocol_type: 0x0800, // IPv4
            hardware_size: 0x06,
            protocol_size: 0x04,
            opcode: 0x0001, // ARP request
            sender_mac: MacAddr::from(sender_mac),
            sender_ip: sender_ip
                .parse::<Ipv4Addr>()
                .expect("invalid sender ip address"),
            target_mac: BROADCAST_MAC_ADDR,
            target_ip: target_ip
                .parse::<Ipv4Addr>()
                .expect("invalid target ip address"),
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
        bytes.extend(self.sender_mac.octets());
        bytes.extend(self.sender_ip.octets());
        bytes.extend(self.target_mac.octets());
        bytes.extend(self.target_ip.octets());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            hardware_type: u16::from_be_bytes([bytes[0], bytes[1]]),
            protocol_type: u16::from_be_bytes([bytes[2], bytes[3]]),
            hardware_size: bytes[4],
            protocol_size: bytes[5],
            opcode: u16::from_be_bytes([bytes[6], bytes[7]]),
            sender_mac: MacAddr::new(
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
            ),
            sender_ip: Ipv4Addr::new(bytes[14], bytes[15], bytes[16], bytes[17]),
            target_mac: MacAddr::new(
                bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            ),
            target_ip: Ipv4Addr::new(bytes[24], bytes[25], bytes[26], bytes[27]),
        }
    }
}
