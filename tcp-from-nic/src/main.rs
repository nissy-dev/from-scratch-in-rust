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
            }
        }
        "http" => {
            let server = http::Server::new();
            server.listen();
            loop {
                let conn = &mut server.accept();
                let req = http::HttpRequest::parse(&conn.1.payload());
                info!("request: {:?}", req);
                match (req.method.as_str(), req.uri.as_str()) {
                    ("GET", "/") => {
                        let res_body = "Hello, World!\r\n";
                        let res = http::HttpResponse::new(http::StatusCode::OK, res_body);
                        server.write(conn, res.to_bytes().as_slice());
                    }
                    _ => (),
                }
            }
        }
        _ => (),
    }
}
