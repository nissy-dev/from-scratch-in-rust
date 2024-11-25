use std::{sync::Arc, thread};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::nic::{NetDevice, Packet};

const IP_VERSION: u8 = 4;
const IHL: u8 = 5;
const TOS: u8 = 0;
const TTL: u8 = 64;
const LENGTH: u8 = IHL * 4;
const TCP_PROTOCOL: u8 = 6;
const HEADER_MIN_LEN: usize = 20;

#[derive(Debug)]
pub struct IpHeader {
    version: u8,
    // Internet Header Length
    ihl: u8,
    // Type of Service (0-5 まで指定できサービスレベル（通常はトラフィックの優先順位）を表現する)
    tos: u8,
    total_length: u16,
    // パケットをフラグメント化する際に使用される識別子
    identification: u16,
    flags: u8,
    fragment_offset: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

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
impl IpHeader {
    pub fn new(src_ip: [u8; 4], dst_ip: [u8; 4], len: u16) -> IpHeader {
        IpHeader {
            version: IP_VERSION,
            ihl: IHL,
            tos: TOS,
            total_length: LENGTH as u16 + len as u16,
            identification: 0,
            flags: 0x40, // フラグメント化を許可しない
            fragment_offset: 0,
            ttl: TTL,
            protocol: TCP_PROTOCOL,
            checksum: 0,
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
            ihl: bytes[0] & 0x0F,
            tos: bytes[1],
            total_length: u16::from_be_bytes([bytes[2], bytes[3]]),
            identification: u16::from_be_bytes([bytes[4], bytes[5]]),
            flags: bytes[6] >> 5,
            fragment_offset: u16::from_be_bytes([bytes[6], bytes[7]]) & 0x1FFF,
            ttl: bytes[8],
            protocol: bytes[9],
            checksum: u16::from_be_bytes([bytes[10], bytes[11]]),
            src_ip: [bytes[12], bytes[13], bytes[14], bytes[15]],
            dst_ip: [bytes[16], bytes[17], bytes[18], bytes[19]],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_MIN_LEN);
        let version_and_ihl = (self.version << 4) | self.ihl;
        let flags_and_fragment_offset = (self.flags << 5) as u16 | (self.fragment_offset & 0x1FFF);
        bytes[0] = version_and_ihl;
        bytes[1] = 0;
        bytes[2..4].copy_from_slice(&self.total_length.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.identification.to_be_bytes());
        bytes[6..8].copy_from_slice(&flags_and_fragment_offset.to_be_bytes());
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.src_ip);
        bytes[16..20].copy_from_slice(&self.dst_ip);
        // calculate checksum
        self.set_checksum(&mut bytes);
        bytes
    }

    pub fn set_checksum(&self, bytes: &mut [u8]) {
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
    pub header: IpHeader,
    packet: Packet,
}

pub struct IpPacketManager {
    incoming_queue: Arc<(Sender<IpPacket>, Receiver<IpPacket>)>,
    outgoing_queue: Arc<(Sender<Packet>, Receiver<Packet>)>,
}

impl IpPacketManager {
    pub fn new() -> IpPacketManager {
        IpPacketManager {
            incoming_queue: Arc::new(bounded::<IpPacket>(10)),
            outgoing_queue: Arc::new(bounded::<Packet>(10)),
        }
    }

    pub fn manage_queue(&self, device: Arc<NetDevice>) {
        let incoming_queue = self.incoming_queue.clone();
        let read_device = device.clone();

        thread::spawn(move || loop {
            let packet = read_device.read();
            let header = IpHeader::from_bytes(&packet.data);
            let ip_packet = IpPacket { header, packet };
            let (sender, _) = incoming_queue.as_ref();
            sender.send(ip_packet).expect("failed to send IP packet");
        });

        let outgoing_queue = self.outgoing_queue.clone();
        let write_device = device.clone();
        thread::spawn(move || loop {
            let (_, out_receiver) = outgoing_queue.as_ref();
            let packet = out_receiver.recv().expect("failed to receive packet");
            write_device.write(packet);
        });
    }

    pub fn read(&self) -> IpPacket {
        let (_, receiver) = self.incoming_queue.as_ref();
        receiver.recv().expect("failed to receive IP packet")
    }

    pub fn write(&self, packet: Packet) {
        let (sender, _) = self.outgoing_queue.as_ref();
        sender.send(packet).expect("failed to send packet");
    }
}
