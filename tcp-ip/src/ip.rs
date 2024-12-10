use crate::nic::{NetDevice, Packet};
use crossbeam_channel::{bounded, Receiver, Sender};
use std::{sync::Arc, thread};

// 基本は 20 byte だが、オプションフィールドがある場合はそれが追加される
pub const IP_HEADER_LENGTH: usize = 20;
const HEADER_MIN_LEN: usize = 20;

// IP ヘッダのフォーマット
// cf: https://datatracker.ietf.org/doc/html/rfc791#section-3.1
//
// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |Version|  IHL  |Type of Service|         Total Length          |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |         Identification        |Flags|   Fragment Offset       |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |  Time to Live |    Protocol   |       Header Checksum         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                        Source Address                         |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                     Destination Address                       |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                    (Options)                    |  (Padding)  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone, Copy)]
pub struct IpHeader {
    version: u8,
    // Internet Header Length (IHL)：ヘッダの長さ
    pub ihl: u8,
    // Type of Service：トラフィックの優先順位を 0-5 の値で指定
    tos: u8,
    // パケット全体の長さ
    total_length: u16,
    // パケットをフラグメント化する際に使用される識別子
    identification: u16,
    // フラグメント化の制御フラグ
    flags: u8,
    // フラグメントのオフセット
    fragment_offset: u16,
    // Time to Live：パケットがネットワーク上に存在できる時間
    ttl: u8,
    protocol: u8,
    checksum: u16,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
}

impl IpHeader {
    pub fn new(src_ip: [u8; 4], dst_ip: [u8; 4], len: usize) -> IpHeader {
        IpHeader {
            version: 4,                        // 常に 4
            ihl: (IP_HEADER_LENGTH / 4) as u8, // 32 ビット単位で表現するため 4 で割る
            tos: 0,                            // 優先度が一番低い 0 を指定
            total_length: (IP_HEADER_LENGTH + len) as u16,
            identification: 0,  // フラグメント化しないので 0
            flags: 2,           // フラグメント化を許可しない (010)
            fragment_offset: 0, // フラグメント化しないので 0
            ttl: 64,            // 64, 128, 255 などを指定、今回は 64
            protocol: 6,        // TCP のプロトコル番号
            checksum: 0,        // 後でセットする
            src_ip,
            dst_ip,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> IpHeader {
        if bytes.len() < HEADER_MIN_LEN {
            panic!("invalid IP header length: {}", bytes.len());
        }
        IpHeader {
            version: bytes[0] >> 4,
            // 0x0F=00001111 で下位 4 ビットを取得
            ihl: bytes[0] & 0x0F,
            tos: bytes[1],
            total_length: u16::from_be_bytes([bytes[2], bytes[3]]),
            identification: u16::from_be_bytes([bytes[4], bytes[5]]),
            flags: bytes[6] >> 5,
            // 0x1FFF=0001111111111111 で下位 13 ビットを取得
            fragment_offset: u16::from_be_bytes([bytes[6], bytes[7]]) & 0x1FFF,
            ttl: bytes[8],
            protocol: bytes[9],
            checksum: u16::from_be_bytes([bytes[10], bytes[11]]),
            src_ip: [bytes[12], bytes[13], bytes[14], bytes[15]],
            dst_ip: [bytes[16], bytes[17], bytes[18], bytes[19]],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let version_and_ihl = (self.version << 4) | self.ihl;
        let flags_and_fragment_offset = ((self.flags as u16) << 13) | (self.fragment_offset & 0x1FFF);
        bytes.push(version_and_ihl);
        bytes.push(self.tos);
        bytes.extend(&self.total_length.to_be_bytes());
        bytes.extend(&self.identification.to_be_bytes());
        bytes.extend(&flags_and_fragment_offset.to_be_bytes());
        bytes.push(self.ttl);
        bytes.push(self.protocol);
        bytes.extend(&self.checksum.to_be_bytes());
        bytes.extend(&self.src_ip);
        bytes.extend(&self.dst_ip);
        // checksum を計算して、再度セットする
        self.set_checksum(&mut bytes);
        bytes
    }

    fn set_checksum(&self, bytes: &mut [u8]) {
        let length = bytes.len();
        let mut checksum = 0u32;
        // パケットの各 2 バイトを 16 ビットの整数として足し合わせる
        for i in (0..length).step_by(2) {
            checksum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
        }
        // 合計が 16 ビットを超えている場合、上位 16 ビットと下位 16 ビットを足し合わせる
        // 0xFFFF は 16 ビットの最大値、checksum >> 16 は上位 16 ビット、checksum & 0xFFFF は下位 16 ビットを取得する
        while checksum > 0xFFFF {
            checksum = (checksum & 0xFFFF) + (checksum >> 16);
        }
        // 1 の補数を取る
        bytes[10..12].copy_from_slice(&(0xFFFF - checksum as u16).to_be_bytes());
    }
}

pub struct IpPacket {
    pub ip_header: IpHeader,
    pub packet: Packet,
}

type Channel = (Sender<IpPacket>, Receiver<IpPacket>);

pub struct IpPacketManager {
    incoming_queue: Arc<Channel>,
    outgoing_queue: Arc<Channel>,
}

impl IpPacketManager {
    pub fn new() -> IpPacketManager {
        IpPacketManager {
            incoming_queue: Arc::new(bounded::<IpPacket>(10)),
            outgoing_queue: Arc::new(bounded::<IpPacket>(10)),
        }
    }

    pub fn manage_queue(&self, device: Arc<NetDevice>) {
        let incoming_queue = self.incoming_queue.clone();
        let read_device = device.clone();

        thread::spawn(move || loop {
            let packet = read_device.read();
            let ip_header = IpHeader::from_bytes(&packet.data);
            let ip_packet = IpPacket { ip_header, packet };
            let (sender, _) = incoming_queue.as_ref();
            sender
                .send(ip_packet)
                .expect("failed to send ip packet in manage_queue");
        });

        let outgoing_queue = self.outgoing_queue.clone();
        let write_device = device.clone();
        thread::spawn(move || loop {
            let (_, out_receiver) = outgoing_queue.as_ref();
            let ip_packet = out_receiver
                .recv()
                .expect("failed to receive ip packet in manage_queue");
            write_device.write(ip_packet.packet);
        });
    }

    pub fn read(&self) -> IpPacket {
        let (_, receiver) = self.incoming_queue.as_ref();
        receiver.recv().expect("failed to receive ip packet")
    }

    pub fn write(&self, ip_packet: IpPacket) {
        let (sender, _) = self.outgoing_queue.as_ref();
        sender.send(ip_packet).expect("failed to send ip packet");
    }
}
