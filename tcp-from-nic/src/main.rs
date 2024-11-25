use std::sync::Arc;

use tracing::info;

mod ip;
mod nic;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "nic" => {
            let nic = nic::NetDevice::new();
            nic.bind();
            loop {
                let packet = nic.read();
                info!("received packet: {:?}", packet);
                nic.write(packet);
            }
        }
        "ip" => {
            let nic = Arc::new(nic::NetDevice::new());
            nic.bind();
            let ip = ip::IpPacketManager::new();
            ip.manage_queue(nic);
            loop {
                let packet = ip.read();
                info!("IP header: {:?}", packet.header);
            }
        }
        _ => (),
    }
}
