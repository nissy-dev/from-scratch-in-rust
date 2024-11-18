use std::net::Ipv4Addr;

use crate::address::MacAddr;

#[derive(Debug)]
pub struct NetworkInterface {
    pub mac_addr: MacAddr,
    pub ip_addr: Ipv4Addr,
    pub ifindex: usize,
}

impl NetworkInterface {
    pub fn new(name: &str) -> Option<NetworkInterface> {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            if ifaddr.interface_name == name {
                if let Some(storage) = ifaddr.address {
                    match (storage.as_link_addr(), storage.as_sockaddr_in()) {
                        (Some(link_addr), Some(sock_addr)) => {
                            let octets = link_addr.addr().unwrap();
                            let mac_addr = MacAddr::new(
                                octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
                            );
                            return Some(NetworkInterface {
                                mac_addr,
                                ip_addr: sock_addr.ip(),
                                ifindex: link_addr.ifindex(),
                            });
                        }
                        _ => return None,
                    }
                }
            }
        }
        None
    }

    pub fn ifindex(&self) -> i32 {
        self.ifindex as i32
    }

    pub fn sll_halen(&self) -> u8 {
        self.mac_addr.octets().len() as u8
    }

    pub fn sll_addr(&self) -> [u8; 8] {
        let octets = self.mac_addr.octets();
        [
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5], 0, 0,
        ]
    }
}
