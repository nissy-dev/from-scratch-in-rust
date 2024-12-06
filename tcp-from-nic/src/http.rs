use crate::{
    ip::IpPacketManager,
    nic::NetDevice,
    tcp::{Connection, HeaderFlags, TcpPacketManager},
};
use std::{collections::HashMap, str, sync::Arc};
use tracing::info;

pub struct Server {
    net_device: Arc<NetDevice>,
    ip_manager: Arc<IpPacketManager>,
    tcp_manager: Arc<TcpPacketManager>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            net_device: Arc::new(NetDevice::new()),
            ip_manager: Arc::new(IpPacketManager::new()),
            tcp_manager: Arc::new(TcpPacketManager::new()),
        }
    }

    pub fn listen(&self) {
        self.net_device.bind();
        self.ip_manager.manage_queue(self.net_device.clone());
        self.tcp_manager.manage_queue(self.ip_manager.clone());
        self.tcp_manager.listen();
        info!("Server is running...");
    }

    pub fn accept(&self) -> Connection {
        self.tcp_manager.accept()
    }

    pub fn write(&self, conn: &mut Connection, data: &[u8]) {
        self.tcp_manager
            .write(conn, HeaderFlags::PSH | HeaderFlags::ACK, data);
    }
}

struct HttpRequest {
    method: String,
    uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn parse(raw: &[u8]) -> HttpRequest {
        let str = str::from_utf8(raw).expect("invalid request data");
        let values = str.split(" ").collect::<Vec<_>>();
        if values.len() < 3 {
            panic!("invalid request data");
        }
        let mut request = HttpRequest {
            method: values[0].to_string(),
            uri: values[1].to_string(),
            version: values[2].to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
    }
}
