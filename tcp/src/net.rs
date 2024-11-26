use std::net::Ipv4Addr;

use tracing::info;

use crate::address::MacAddr;

#[derive(Debug)]
pub struct NetworkInterface {
    pub mac_addr: MacAddr,
    pub ip_addr: Ipv4Addr,
    pub ifindex: usize,
}

impl NetworkInterface {
    pub fn new(name: &str) -> Option<NetworkInterface> {
        let mut mac_addr = None;
        let mut ip_addr = None;
        let mut ifindex = None;
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            if ifaddr.interface_name == name {
                if let Some(storage) = ifaddr.address {
                    if let Some(link_addr) = storage.as_link_addr() {
                        ifindex = Some(link_addr.ifindex());
                        if let Some(octets) = link_addr.addr() {
                            mac_addr = Some(MacAddr::new(
                                octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
                            ));
                        }
                    }
                    if let Some(sock_addr) = storage.as_sockaddr_in() {
                        ip_addr = Some(sock_addr.ip());
                    }
                }
            }
        }
        info!(
            "mac_addr='{:?}', ip_addr='{:?}', ifindex='{:?}'",
            mac_addr?, ip_addr?, ifindex?
        );
        match (mac_addr, ip_addr, ifindex) {
            (Some(mac_addr), Some(ip_addr), Some(ifindex)) => Some(NetworkInterface {
                mac_addr,
                ip_addr,
                ifindex,
            }),
            _ => None,
        }
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
