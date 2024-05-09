use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::ops::Range;

use anyhow::{Ok, Result};
use rand::rngs::ThreadRng;

use crate::{
    socket::{SockID, Socket},
    tcpflags,
};

const UNDETERMINED_IP_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const UNDETERMINED_PORT: u16 = 0;
const MAX_TRANSMITTION: u8 = 5;
const RETRANSMITTION_TIMEOUT: u64 = 3;
const MSS: usize = 1460;
const PORT_RANGE: Range<u16> = 40000..60000;

pub struct TCP {
    sockets: HashMap<SockID, Socket>,
}

impl TCP {
    pub fn new() -> Self {
        let sockets = HashMap::new();
        Self { sockets }
    }

    fn select_unused_port(&self, rng: &mut ThreadRng) -> Result<u16> {
        Ok(33445)
    }

    pub fn connect(&self, addr: Ipv4Addr, port: u16) -> Result<SockID> {
        let mut rng = rand::thread_rng();
        let mut socket = Socket::new(
            get_source_addr_to(addr)?,
            addr,
            self.select_unused_port(&mut rng)?,
            port,
        )?;
        socket.send_tcp_packet(tcpflags::SYN, &[])?;
        let sock_id = socket.get_sock_id();
        Ok(sock_id)
    }
}

fn get_source_addr_to(addr: Ipv4Addr) -> Result<Ipv4Addr> {
    Ok("10.0.0.1".parse().unwrap())
}
