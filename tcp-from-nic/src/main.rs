use std::sync::Arc;

use tracing::info;

mod http;
mod ip;
mod nic;
mod tcp;

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
            }
        }
        "ip" => {
            let nic = Arc::new(nic::NetDevice::new());
            nic.bind();
            let ip = ip::IpPacketManager::new();
            ip.manage_queue(nic);
            loop {
                let packet = ip.read();
                info!("IP header: {:?}", packet.ip_header);
            }
        }
        "tcp" => {
            let nic = Arc::new(nic::NetDevice::new());
            nic.bind();
            let ip = Arc::new(ip::IpPacketManager::new());
            ip.manage_queue(nic);
            let tcp = tcp::TcpPacketManager::new();
            tcp.manage_queue(ip);
            tcp.listen();
            loop {
                let conn = tcp.accept();
                info!("TCP connection: {:?}", conn);
                // let flag = tcp::HeaderFlags::ACK;
                // let data = vec![0; 10];
                // tcp.write(&mut conn, flag, data);
            }
        }
        _ => (),
    }
}
