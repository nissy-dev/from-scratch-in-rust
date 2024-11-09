// use std::mem;

// use libc::{sockaddr_ll, AF_PACKET, ARPHRD_ETHER, ETH_P_ALL, ETH_P_ARP, SOCK_RAW};

// use crate::{
//     syscall,
//     utils::{IpAddr, MacAddr},
// };

// struct Arp {
//     pub hardware_type: u16,
//     pub protocol_type: u16,
//     pub hardware_size: u8,
//     pub protocol_size: u8,
//     pub opcode: u16,
//     pub sender_mac: MacAddr,
//     pub sender_ip: IpAddr,
//     pub target_mac: MacAddr,
//     pub target_ip: IpAddr,
// }

// impl Arp {
//     pub fn new(&self, sender_mac: &str, sender_ip: &str, target_ip: &str) -> Self {
//         Arp {
//             hardware_type: 0x0001,
//             protocol_type: 0x0800, // IPv4
//             hardware_size: 0x06,
//             protocol_size: 0x04,
//             opcode: 0x0001,
//             sender_mac: self.mac_addr_to_bytes(sender_mac),
//             sender_ip: self.ip_addr_to_bytes(sender_ip),
//             target_mac: [0; 6], // Broadcast
//             target_ip: self.ip_addr_to_bytes(target_ip),
//         }
//     }

//     fn mac_addr_to_bytes(&self, mac_addr: &str) -> MacAddr {
//         let mut bytes = [0; 6];
//         for (i, byte_str) in mac_addr.split(':').enumerate() {
//             bytes[i] = u8::from_str_radix(byte_str, 16).unwrap();
//         }
//         bytes
//     }

//     fn ip_addr_to_bytes(&self, ip_addr: &str) -> IpAddr {
//         let mut bytes = [0; 4];
//         for (i, byte_str) in ip_addr.split('.').enumerate() {
//             bytes[i] = byte_str.parse().unwrap();
//         }
//         bytes
//     }

//     fn is_arp_packet(&self, packet: &[u8]) -> bool {
//         packet[12..14] == [0x08, 0x06]
//     }

//     fn from_bytes(bytes: &[u8]) -> Self {
//         Self {
//             hardware_type: u16::from_be_bytes([bytes[0], bytes[1]]),
//             protocol_type: u16::from_be_bytes([bytes[2], bytes[3]]),
//             hardware_size: bytes[4],
//             protocol_size: bytes[5],
//             opcode: u16::from_be_bytes([bytes[6], bytes[7]]),
//             sender_mac: [
//                 bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
//             ],
//             sender_ip: [bytes[14], bytes[15], bytes[16], bytes[17]],
//             target_mac: [
//                 bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
//             ],
//             target_ip: [bytes[24], bytes[25], bytes[26], bytes[27]],
//         }
//     }

//     pub fn send(&self, ifindex: i32, packet: &[u8]) -> Arp {
//         let addr = sockaddr_ll {
//             sll_family: AF_PACKET as u16,
//             sll_protocol: ETH_P_ARP as u16,
//             sll_ifindex: ifindex,
//             sll_hatype: ARPHRD_ETHER as u16,
//             sll_pkttype: 0,
//             sll_halen: 0,
//             sll_addr: [0; 8],
//         };
//         let fd = syscall::socket(AF_PACKET, SOCK_RAW, ETH_P_ALL);
//         syscall::sendto(fd, packet, &addr);

//         loop {
//             let mut buf = [0; 80];
//             syscall::recvfrom(fd, &mut buf);
//             if self.is_arp_packet(&buf) {
//                 return Arp::from_bytes(&buf[14..]);
//             }
//         }
//     }
// }
