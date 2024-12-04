use crate::{
    ip::{IpHeader, IpPacket, IpPacketManager, IP_HEADER_LENGTH},
    nic::Packet,
};
use bitflags::bitflags;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tracing::info;

// 基本は 20 byte だが、オプションフィールドがある場合はそれが追加される
const TCP_HEADER_LENGTH: usize = 20;

// TCP のヘッダーフォーマット
// cf: https://datatracker.ietf.org/doc/html/rfc9293#name-header-format
//
// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          Source Port          |       Destination Port        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                        Sequence Number                        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                    Acknowledgment Number                      |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |  Data |       |C|E|U|A|P|R|S|F|                               |
// | Offset| Rsrvd |W|C|R|C|S|S|Y|I|            Window             |
// |       |       |R|E|G|K|H|T|N|N|                               |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |           Checksum            |         Urgent Pointer        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           [Options]                           |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

bitflags! {
  #[derive(Debug, Clone, Copy, PartialEq)]
  struct HeaderFlags: u8 {
      const FIN = 1;
      const SYN = 1 << 1;
      const RST = 1 << 2;
      const PSH = 1 << 3;
      const ACK = 1 << 4;
      const URG = 1 << 5;
      const ECE = 1 << 6;
      const CWR = 1 << 7;
  }
}

#[derive(Debug)]
pub struct TcpHeader {
    src_port: u16,
    dst_port: u16,
    // シーケンス番号
    // 最初に送るパケット: ランダムな値, その後: データを送った分だけ増加
    // 3way ハンドシェイク時はデータサイズは 0 だが、 SYN / FIN パケットを送ったときは 1 増加させる
    seq_num: u32,
    // 確認応答番号
    // 最初に送る SYN パケット: 0、その後: 「相手から受信したシーケンス番号」＋「受け取ったデータサイズの値」
    // 3way ハンドシェイク時はデータサイズは 0 だが、 SYN / FIN パケットを受け取ったときは 1 増加させる
    ack_num: u32,
    // TCP ヘッダの長さ
    data_offset: u8,
    // 将来のために予約されているフィールド、通常 0
    reserved: u8,
    flag: HeaderFlags,
    window: u16,
    checksum: u16,
    urgent_pointer: u16,
}

impl TcpHeader {
    fn new(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        flag: HeaderFlags,
    ) -> TcpHeader {
        TcpHeader {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            flag,
            data_offset: (TCP_HEADER_LENGTH / 4) as u8, // 32 ビット単位で表現するため 4 で割る
            reserved: 0,                                // 将来のためのフィールド、0 で初期化
            window: 65535,                              // 最大値を指定
            checksum: 0,                                // 後でセットする
            urgent_pointer: 0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> TcpHeader {
        TcpHeader {
            src_port: u16::from_be_bytes([bytes[0], bytes[1]]),
            dst_port: u16::from_be_bytes([bytes[2], bytes[3]]),
            seq_num: u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            ack_num: u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            data_offset: bytes[12] >> 4,
            // 0x0F=00001111 で下位 4 ビットを取得
            reserved: bytes[12] & 0x0F,
            flag: HeaderFlags::from_bits_truncate(bytes[13]),
            window: u16::from_be_bytes([bytes[14], bytes[15]]),
            checksum: u16::from_be_bytes([bytes[16], bytes[17]]),
            urgent_pointer: u16::from_be_bytes([bytes[18], bytes[19]]),
        }
    }

    fn to_bytes(&self, ip_header: &IpHeader, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.src_port.to_be_bytes());
        bytes.extend(&self.dst_port.to_be_bytes());
        bytes.extend(&self.seq_num.to_be_bytes());
        bytes.extend(&self.ack_num.to_be_bytes());
        bytes.push(self.data_offset << 4 | self.reserved);
        bytes.push(self.flag.bits());
        bytes.extend(&self.window.to_be_bytes());
        bytes.extend(&self.checksum.to_be_bytes());
        bytes.extend(&self.urgent_pointer.to_be_bytes());
        // checksum を計算して、再度セットする
        self.set_checksum(&mut bytes, ip_header, data);
        bytes
    }

    fn set_checksum(&self, bytes: &mut [u8], ip_header: &IpHeader, data: &[u8]) {
        let mut pseudo_header = [0; 12];
        pseudo_header[0..4].copy_from_slice(&ip_header.src_ip);
        pseudo_header[4..8].copy_from_slice(&ip_header.dst_ip);
        pseudo_header[8] = 0;
        pseudo_header[9] = 6; // TCP のプロトコル番号
        pseudo_header[10..12].copy_from_slice(&(bytes.len() as u16).to_be_bytes());

        let mut tmp_buf = [pseudo_header.to_vec(), bytes.to_vec(), data.to_vec()].concat();
        if tmp_buf.len() % 2 != 0 {
            tmp_buf.push(0);
        }

        let length = tmp_buf.len();
        let mut checksum = 0u32;
        // パケットの各 2 バイトを 16 ビットの整数として足し合わせる
        for i in (0..length).step_by(2) {
            checksum += u16::from_be_bytes([tmp_buf[i], tmp_buf[i + 1]]) as u32;
        }
        // 合計が 16 ビットを超えている場合、上位 16 ビットと下位 16 ビットを足し合わせる
        // 0xFFFF は 16 ビットの最大値、checksum >> 16 は上位 16 ビット、checksum & 0xFFFF は下位 16 ビットを取得する
        while checksum > 0xFFFF {
            checksum = (checksum & 0xFFFF) + (checksum >> 16);
        }
        // 1 の補数を取る
        bytes[16..18].copy_from_slice(&(0xFFFF - checksum as u16).to_be_bytes());
    }
}

// TCP の状態遷移図
// cf: https://datatracker.ietf.org/doc/html/rfc9293#section-3.3.2
//
//                               +---------+ ---------\      active OPEN
//                               |  CLOSED |            \    -----------
//                               +---------+<---------\   \   create TCB
//                                 |     ^              \   \  snd SYN
//                    passive OPEN |     |   CLOSE        \   \
//                    ------------ |     | ----------       \   \
//                     create TCB  |     | delete TCB         \   \
//                                 V     |                      \   \
//                               +---------+            CLOSE    |    \
//                               |  LISTEN |          ---------- |     |
//                               +---------+          delete TCB |     |
//                    rcv SYN      |     |     SEND              |     |
//                   -----------   |     |    -------            |     V
//  +---------+      snd SYN,ACK  /       \   snd SYN          +---------+
//  |         |<-----------------           ------------------>|         |
//  |   SYN   |                    rcv SYN                     |   SYN   |
//  |   RCVD  |<-----------------------------------------------|   SENT  |
//  |         |                    snd ACK                     |         |
//  |         |------------------           -------------------|         |
//  +---------+   rcv ACK of SYN  \       /  rcv SYN,ACK       +---------+
//    |           --------------   |     |   -----------
//    |                  x         |     |     snd ACK
//    |                            V     V
//    |  CLOSE                   +---------+
//    | -------                  |  ESTAB  |
//    | snd FIN                  +---------+
//    |                   CLOSE    |     |    rcv FIN
//    V                  -------   |     |    -------
//  +---------+          snd FIN  /       \   snd ACK          +---------+
//  |  FIN    |<-----------------           ------------------>|  CLOSE  |
//  | WAIT-1  |------------------                              |   WAIT  |
//  +---------+          rcv FIN  \                            +---------+
//    | rcv ACK of FIN   -------   |                            CLOSE  |
//    | --------------   snd ACK   |                           ------- |
//    V        x                   V                           snd FIN V
//  +---------+                  +---------+                   +---------+
//  |FINWAIT-2|                  | CLOSING |                   | LAST-ACK|
//  +---------+                  +---------+                   +---------+
//    |                rcv ACK of FIN |                 rcv ACK of FIN |
//    |  rcv FIN       -------------- |    Timeout=2MSL -------------- |
//    |  -------              x       V    ------------        x       V
//     \ snd ACK                 +---------+delete TCB         +---------+
//      ------------------------>|TIME WAIT|------------------>| CLOSED  |
//                               +---------+                   +---------+

#[derive(Debug)]
pub struct TcpPacket {
    ip_header: IpHeader,
    tcp_header: TcpHeader,
    packet: Packet,
}

#[derive(Debug, Clone, Copy)]
pub struct Connection {
    src_port: u16,
    dst_port: u16,
    state: ConnectionState,
    next_seq_num: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionState {
    Listen,
    SynReceived,
    Established,
    CloseWait,
    LastAck,
    Closed,
}

struct ConnectionManager {
    connections: Mutex<Vec<Connection>>,
    accpted_connections: (Sender<Connection>, Receiver<Connection>),
}

impl ConnectionManager {
    pub fn new() -> ConnectionManager {
        ConnectionManager {
            connections: Mutex::new(Vec::new()),
            accpted_connections: bounded::<Connection>(10),
        }
    }

    // TCP passive open の流れを実装
    pub fn passive_handler(&self, incoming_queue: &Channel, outgoing_queue: &Channel) {
        let (_, receiver) = incoming_queue;
        let incoming_packet = receiver
            .recv()
            .expect("failed to receive tcp packet in receive_handler");

        // コネクションが存在するか確認し、存在しない場合は新規作成する
        self.ensure_connection_exsists(&incoming_packet);
        let mut connections = self.connections.lock().unwrap();
        let connection = connections
            .iter_mut()
            .find(|c| {
                c.src_port == incoming_packet.tcp_header.src_port
                    && c.dst_port == incoming_packet.tcp_header.dst_port
            })
            .expect("failed to find connection");

        let flag = incoming_packet.tcp_header.flag;
        let (sender, _) = outgoing_queue;
        info!("connection state: {:?}, flag: {:?}", connection.state, flag);
        match flag {
            _ if flag.contains(HeaderFlags::SYN) && connection.state == ConnectionState::Listen => {
                info!("received SYN packet...");
                self.send_packet(
                    sender,
                    connection,
                    &incoming_packet,
                    HeaderFlags::SYN | HeaderFlags::ACK,
                    &[],
                );
                connection.state = ConnectionState::SynReceived;
            }
            _ if flag.contains(HeaderFlags::ACK)
                && connection.state == ConnectionState::SynReceived =>
            {
                info!("received ACK packet...");
                connection.state = ConnectionState::Established;
            }
            _ if flag.contains(HeaderFlags::PSH)
                && connection.state == ConnectionState::Established =>
            {
                info!("received PSH packet...");
                self.send_packet(sender, connection, &incoming_packet, HeaderFlags::ACK, &[]);
                let (sender, _) = &self.accpted_connections;
                sender
                    .send(*connection)
                    .expect("failed to send connection in passive_handler");
            }
            _ if flag.contains(HeaderFlags::FIN)
                && connection.state == ConnectionState::Established =>
            {
                info!("received FIN packet...");
                self.send_packet(sender, connection, &incoming_packet, HeaderFlags::ACK, &[]);
                connection.state = ConnectionState::CloseWait;

                // RFC を読むと FIN パケットを送るように書いてあるが、FIN/ACK を送ることが想定されているらしい
                // cf: https://kawasin73.hatenablog.com/entry/2019/08/31/153809
                self.send_packet(
                    sender,
                    connection,
                    &incoming_packet,
                    HeaderFlags::FIN | HeaderFlags::ACK,
                    &[],
                );
                connection.state = ConnectionState::LastAck;
            }
            _ if flag.contains(HeaderFlags::ACK)
                && connection.state == ConnectionState::LastAck =>
            {
                info!("received ACK packet...");
                connection.state = ConnectionState::Closed;
                info!("connection closed");
            }
            _ => {
                info!("unexpected condition");
            }
        }
    }

    pub fn ensure_connection_exsists(&self, tcp_packet: &TcpPacket) {
        let mut connections = self.connections.lock().unwrap();
        // Close したコネクションを削除する
        connections.retain(|c| c.state != ConnectionState::Closed);

        // コネクションが存在するか確認し、存在しない場合は新規作成する
        let conn = connections.iter().find(|c| {
            c.src_port == tcp_packet.tcp_header.src_port
                && c.dst_port == tcp_packet.tcp_header.dst_port
        });
        if conn.is_none() {
            connections.push(Connection {
                src_port: tcp_packet.tcp_header.src_port,
                dst_port: tcp_packet.tcp_header.dst_port,
                next_seq_num: 0,
                state: ConnectionState::Listen,
            });
        }
    }

    pub fn send_packet(
        &self,
        packet_sender: &Sender<TcpPacket>,
        connection: &mut Connection,
        incoming_packet: &TcpPacket,
        outgoing_packet_flag: HeaderFlags,
        outgoing_packet_data: &[u8],
    ) {
        // IP ヘッダーの生成
        let ip_header = IpHeader::new(
            incoming_packet.ip_header.dst_ip,
            incoming_packet.ip_header.src_ip,
            TCP_HEADER_LENGTH + outgoing_packet_data.len(),
        );

        // TCP ヘッダーの生成
        let increment_ack_num = (if outgoing_packet_flag.contains(HeaderFlags::SYN)
            || outgoing_packet_flag.contains(HeaderFlags::FIN)
        {
            1
        } else {
            incoming_packet.packet.data.len()
                - (incoming_packet.ip_header.ihl * 4) as usize
                - (incoming_packet.tcp_header.data_offset * 4) as usize
        }) as u32;
        let tcp_header = TcpHeader::new(
            incoming_packet.tcp_header.dst_port,
            incoming_packet.tcp_header.src_port,
            connection.next_seq_num,
            incoming_packet.tcp_header.seq_num + increment_ack_num,
            outgoing_packet_flag,
        );

        // パケットの生成
        let packet = Packet {
            data: [
                ip_header.to_bytes(),
                tcp_header.to_bytes(&ip_header, outgoing_packet_data),
                outgoing_packet_data.to_vec(),
            ]
            .concat(),
        };

        // TCP パケットの生成
        let tcp_packet = TcpPacket {
            ip_header: IpHeader::from_bytes(&packet.data[..IP_HEADER_LENGTH]),
            tcp_header: TcpHeader::from_bytes(
                &packet.data[IP_HEADER_LENGTH..(IP_HEADER_LENGTH + TCP_HEADER_LENGTH)],
            ),
            packet,
        };
        info!("send packet: {:?}", tcp_packet);
        packet_sender
            .send(tcp_packet)
            .expect("failed to send tcp packet in send_packet");

        // 次のシーケンス番号を計算する
        let increment_seq_num = (if outgoing_packet_flag.contains(HeaderFlags::SYN)
            || outgoing_packet_flag.contains(HeaderFlags::FIN)
        {
            1
        } else {
            outgoing_packet_data.len()
        }) as u32;
        connection.next_seq_num += increment_seq_num;
    }
}

type Channel = (Sender<TcpPacket>, Receiver<TcpPacket>);

pub struct TcpPacketManager {
    connection_manager: Arc<ConnectionManager>,
    incoming_queue: Arc<Channel>,
    outgoing_queue: Arc<Channel>,
}

impl TcpPacketManager {
    pub fn new() -> TcpPacketManager {
        TcpPacketManager {
            connection_manager: Arc::new(ConnectionManager::new()),
            incoming_queue: Arc::new(bounded::<TcpPacket>(10)),
            outgoing_queue: Arc::new(bounded::<TcpPacket>(10)),
        }
    }

    pub fn manage_queue(&self, ip_manager: Arc<IpPacketManager>) {
        let read_ip_manager = ip_manager.clone();
        let incoming_queue = self.incoming_queue.clone();
        thread::spawn(move || loop {
            let ip_packet = read_ip_manager.read();
            let offset = (ip_packet.ip_header.ihl * 4) as usize;
            let tcp_packet = TcpPacket {
                ip_header: ip_packet.ip_header,
                tcp_header: TcpHeader::from_bytes(&ip_packet.packet.data[offset..]),
                packet: ip_packet.packet,
            };
            let (sender, _) = incoming_queue.as_ref();
            sender
                .send(tcp_packet)
                .expect("failed to send tcp packet in manage_queue");
        });

        let write_ip_manager = ip_manager.clone();
        let outgoing_queue = self.outgoing_queue.clone();
        thread::spawn(move || loop {
            let (_, receiver) = outgoing_queue.as_ref();
            let tcp_packet = receiver
                .recv()
                .expect("failed to receive tcp packet in manage_queue");

            let ip_packet = IpPacket {
                ip_header: tcp_packet.ip_header,
                packet: tcp_packet.packet,
            };
            write_ip_manager.write(ip_packet);
        });
    }

    pub fn listen(&self) {
        let connection_manager = self.connection_manager.clone();
        let incoming_queue = self.incoming_queue.clone();
        let outgoing_queue = self.outgoing_queue.clone();
        thread::spawn(move || loop {
            connection_manager.passive_handler(incoming_queue.as_ref(), outgoing_queue.as_ref());
        });
    }

    pub fn accept(&self) -> Connection {
        let (_, receiver) = &self.connection_manager.accpted_connections;
        receiver
            .recv()
            .expect("failed to receive connection in accept")
    }
}
