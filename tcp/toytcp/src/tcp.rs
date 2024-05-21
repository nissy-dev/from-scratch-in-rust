use std::net::{IpAddr, Ipv4Addr};
use std::ops::Range;
use std::process::Command;
use std::str;
use std::sync::{Arc, Condvar, Mutex};
use std::{collections::HashMap, sync::RwLock};

use anyhow::{Context, Result};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::Packet;
use pnet::transport;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::packet::TCPPacket;
use crate::socket::TcpStatus;
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
    sockets: RwLock<HashMap<SockID, Socket>>,
    event_condvar: (Mutex<Option<TCPEvent>>, Condvar),
}

#[derive(Debug, Clone, PartialEq)]
struct TCPEvent {
    sock_id: SockID,
    kind: TCPEventKind,
}

impl TCPEvent {
    fn new(sock_id: SockID, kind: TCPEventKind) -> Self {
        Self { sock_id, kind }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TCPEventKind {
    ConnectionComplted,
    Acked,
    DataArrived,
    ConnectionClosed,
}

impl TCP {
    pub fn new() -> Arc<Self> {
        let sockets = RwLock::new(HashMap::new());
        let tcp = Arc::new(Self {
            sockets,
            event_condvar: (Mutex::new(None), Condvar::new()),
        });
        let cloned_tcp = tcp.clone();
        std::thread::spawn(move || {
            // パケットを受信する
            cloned_tcp.receive_handler().unwrap();
        });
        tcp
    }

    // ランダムに未使用なポートを選択する
    fn select_unused_port(&self, rng: &mut ThreadRng) -> Result<u16> {
        for _ in 0..(PORT_RANGE.end - PORT_RANGE.start) {
            let local_port = rng.gen_range(PORT_RANGE);
            let table = self.sockets.read().unwrap();
            if table.keys().all(|k| local_port != k.2) {
                return Ok(local_port);
            }
        }
        anyhow::bail!("no available port found");
    }

    pub fn connect(&self, addr: Ipv4Addr, port: u16) -> Result<SockID> {
        let mut rng = rand::thread_rng();
        let mut socket = Socket::new(
            get_source_addr_to(addr)?,
            addr,
            self.select_unused_port(&mut rng)?,
            port,
            TcpStatus::SynSent,
        )?;
        // セキュリティのため乱数で初期シーケンス番号を設定する
        // 参考: https://ja.wikipedia.org/wiki/TCP%E3%82%B7%E3%83%BC%E3%82%B1%E3%83%B3%E3%82%B9%E7%95%AA%E5%8F%B7%E4%BA%88%E6%B8%AC%E6%94%BB%E6%92%83
        socket.send_param.initial_seq = rng.gen_range(1..1 << 31);
        // SYNフラグを立てて接続要求を送信する
        socket.send_tcp_packet(socket.send_param.initial_seq, 0, tcpflags::SYN, &[])?;
        socket.send_param.unpacked_seq = socket.send_param.initial_seq;
        socket.send_param.next = socket.send_param.initial_seq + 1;

        let mut table = self.sockets.write().unwrap();
        let sock_id = socket.get_sock_id();
        table.insert(sock_id, socket);
        // ロックを外してイベントの待機．受信スレッドがロックを取得できるようにするため．
        drop(table);
        dbg!("wait connection completed");
        self.wait_event(sock_id, TCPEventKind::ConnectionComplted);
        Ok(sock_id)
    }

    fn receive_handler(&self) -> Result<()> {
        dbg!("begin recv thread");
        let (_, mut receiver) = transport::transport_channel(
            65535,
            transport::TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp),
        )
        .unwrap();

        let mut packet_iter = transport::ipv4_packet_iter(&mut receiver);

        loop {
            dbg!("loop join");
            let (packet, remote_addr) = match packet_iter.next() {
                Ok((p, r)) => (p, r),
                Err(_) => continue,
            };
            let local_addr = packet.get_destination();
            // pnet の TcpPacket を作成する
            let tcp_packet = match TcpPacket::new(packet.payload()) {
                Some(p) => p,
                None => {
                    continue;
                }
            };
            let packet = TCPPacket::from(tcp_packet);
            let remote_addr = match remote_addr {
                IpAddr::V4(addr) => addr,
                _ => {
                    continue;
                }
            };
            dbg!("packet", &packet);
            let mut table = self.sockets.write().unwrap();
            let socket = match table.get_mut(&SockID(
                local_addr,
                remote_addr,
                packet.get_dest(),
                packet.get_src(),
            )) {
                // コネクション成立済みのソケットがあるとき
                Some(socket) => socket,
                // コネクション成立済みのソケットがないとき
                None => match table.get_mut(&SockID(
                    local_addr,
                    UNDETERMINED_IP_ADDR,
                    packet.get_dest(),
                    UNDETERMINED_PORT,
                )) {
                    // リスニングソケット (リクエストを待っているソケット) があるかどうか確認する
                    Some(socket) => socket, // リスニングソケット
                    None => continue,       // どのソケットにも該当しないものは無視する
                },
            };
            if !packet.is_correct_checksum(local_addr, remote_addr) {
                dbg!("invalid checksum");
                continue;
            }
            let sock_id = socket.get_sock_id();
            if let Err(error) = match socket.status {
                TcpStatus::SynSent => self.syn_sent_handler(socket, &packet),
                _ => {
                    dbg!("not implemented state");
                    Ok(())
                }
            } {
                dbg!(error);
            }
        }
    }

    // SYNSENT 状態のソケットに到着したパケットを処理する
    fn syn_sent_handler(&self, socket: &mut Socket, packet: &TCPPacket) -> Result<()> {
        dbg!("syn sent handler");
        // アクティブオープンの場合において...
        // SYN パケットを送信して SYNSENT の状態に遷移後、SYN ACK のフラグがたったパケットを受け取る
        if packet.get_flag() & tcpflags::ACK > 0
            && socket.send_param.unpacked_seq <= packet.get_ack()
            && packet.get_ack() < socket.send_param.next
            && packet.get_flag() & tcpflags::SYN > 0
        {
            //
            socket.recv_param.next = packet.get_seq() + 1;
            socket.recv_param.initial_seq = packet.get_ack();
            socket.send_param.unpacked_seq = packet.get_ack();
            socket.send_param.window = packet.get_window_size();
            if socket.send_param.unpacked_seq > socket.send_param.initial_seq {
                // ACK フラグを立てて接続が確立したことを通知する
                socket.status = TcpStatus::Established;
                socket.send_tcp_packet(
                    socket.send_param.next,
                    socket.recv_param.next,
                    tcpflags::ACK,
                    &[],
                )?;
                dbg!("status: syn sent -> ", &socket.status);
                self.publish_event(socket.get_sock_id(), TCPEventKind::ConnectionComplted);
            } else {
                // ACK と SYN フラグを立てて接続要求を返す
                // これはどういうとき？
                socket.status = TcpStatus::SynRcvd;
                socket.send_tcp_packet(
                    socket.send_param.next,
                    socket.recv_param.next,
                    tcpflags::ACK | tcpflags::SYN,
                    &[],
                )?;
                dbg!("status: syn sent -> ", &socket.status);
            }
        }
        Ok(())
    }

    fn wait_event(&self, sock_id: SockID, kind: TCPEventKind) {
        let (lock, cvar) = &self.event_condvar;
        let mut event = lock.lock().unwrap();
        loop {
            if let Some(ref e) = *event {
                if e.sock_id == sock_id && e.kind == kind {
                    break;
                }
            }
            // cvar が notify されるまで event のロックを外して待機する
            event = cvar.wait(event).unwrap();
        }
        dbg!(&event);
        *event = None;
    }

    // 指定のソケット ID にイベントを発行する
    fn publish_event(&self, sock_id: SockID, kind: TCPEventKind) {
        let (lock, cvar) = &self.event_condvar;
        let mut event = lock.lock().unwrap();
        *event = Some(TCPEvent::new(sock_id, kind));
        cvar.notify_all();
    }
}

// 宛先 IP アドレスに対する送信元インターフェイスの IP アドレスを取得する
fn get_source_addr_to(addr: Ipv4Addr) -> Result<Ipv4Addr> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("ip route get {} | grep src", addr))
        .output()?;
    let mut output = str::from_utf8(&output.stdout)?
        .trim()
        .split_ascii_whitespace();
    while let Some(s) = output.next() {
        if s == "src" {
            break;
        }
    }
    let ip = output.next().context("failed to get source ip address")?;
    dbg!("source ip address", ip);
    ip.parse().context("failed to parse ip address")
}
