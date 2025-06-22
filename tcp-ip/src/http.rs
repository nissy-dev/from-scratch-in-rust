use crate::{
    ip::IpPacketManager,
    nic::NetDevice,
    tcp::{HeaderFlags, SharedConnection, TcpPacketManager},
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

    pub fn accept(&self) -> SharedConnection {
        self.tcp_manager.accept()
    }

    pub fn write(&self, conn: &mut SharedConnection, data: &[u8]) {
        self.tcp_manager
            .write(conn, HeaderFlags::PSH | HeaderFlags::ACK, data)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub uri: String,
    version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpRequest {
    pub fn parse(raw: &[u8]) -> HttpRequest {
        let str = str::from_utf8(raw).expect("invalid request data");
        let mut lines = str.lines();

        // リクエストラインの解析
        let request_line = lines.next().expect("invalid request data");
        let values = request_line.split(" ").collect::<Vec<_>>();
        if values.len() < 3 {
            panic!("unexpected request line");
        }
        let mut request = HttpRequest {
            method: values[0].to_string(),
            uri: values[1].to_string(),
            version: values[2].to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };

        // ヘッダの解析
        for line in lines {
            if line.is_empty() {
                break;
            }
            let values = line.split(": ").collect::<Vec<_>>();
            if values.len() < 2 {
                panic!("unexpected header line");
            }
            request
                .headers
                .insert(values[0].to_string(), values[1].to_string());
        }

        // リクエストボディの取得
        // GET は通常リクエストボディを低められないが、実際のところは実装依存らしい
        // cf: https://zenn.dev/kasuna/articles/4c969ec557c5fc
        // Content-Length ヘッダがある場合、ボディを取得する
        if let Some(length) = request.headers.get("Content-Length") {
            let length = length.parse::<usize>().expect("invalid Content-Length");
            request.body = raw[raw.len() - length..].to_vec();
        }

        // ボディの解析
        request
    }
}

pub enum StatusCode {
    OK,
}

impl StatusCode {
    fn to_str(&self) -> &str {
        match self {
            StatusCode::OK => "200 OK",
        }
    }
}

pub struct HttpResponse {
    version: String,
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpResponse {
    pub fn new(status_code: StatusCode, body: &str) -> HttpResponse {
        HttpResponse {
            version: "HTTP/1.1".to_string(),
            status_code,
            headers: HashMap::from([
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Content-Length".to_string(), body.len().to_string()),
            ]),
            body: body.to_string(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(format!("{} {}\r\n", self.version, self.status_code.to_str()).as_bytes());
        for (key, value) in &self.headers {
            bytes.extend(format!("{}: {}\r\n", key, value).as_bytes());
        }
        bytes.extend("\r\n".as_bytes());
        bytes.extend(self.body.as_bytes());
        bytes
    }
}
