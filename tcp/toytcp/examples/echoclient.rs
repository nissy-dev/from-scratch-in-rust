use anyhow::Result;
use std::net::Ipv4Addr;
use toytcp::tcp::TCP;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    let addr: Ipv4Addr = args[1].parse()?;
    let port: u16 = args[2].parse()?;
    echo_client(addr, port)?;
    Ok(())
}

fn echo_client(remote_addr: Ipv4Addr, remote_port: u16) -> Result<()> {
    let client = TCP::new();
    let _ = client.connect(remote_addr, remote_port)?;
    Ok(())
}
