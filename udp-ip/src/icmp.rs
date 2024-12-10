use crate::{
    address::MacAddr,
    ethernet::{EthernetFrame, EthernetType, ETHERNET_FRAME_LENGTH},
    ip::{IpHeader, Protocol, IP_HEADER_LENGTH},
    net::NetworkInterface,
    socket,
};
use std::net::Ipv4Addr;
use tracing::info;

pub const ICMP_HEADER_LENGTH: usize = 8;

pub struct Icmp {}

impl Icmp {
    pub fn send(
        dst_ip_addr: Ipv4Addr,
        dst_mac_addr: MacAddr,
        src_net_interface: &NetworkInterface,
    ) -> Option<IcmpFrame> {
        let packet_length = (IP_HEADER_LENGTH + ICMP_HEADER_LENGTH) as u16;
        let ethernet_frame =
            EthernetFrame::new(EthernetType::Ipv4, dst_mac_addr, src_net_interface.mac_addr);
        let ip_header = IpHeader::new(
            src_net_interface.ip_addr,
            dst_ip_addr,
            packet_length,
            Protocol::IP,
        );
        let icmp_frame = IcmpFrame::new(MessageType::Echo);
        let send_packet = [
            ethernet_frame.to_bytes(),
            ip_header.to_bytes(),
            icmp_frame.to_bytes(),
        ]
        .concat();

        let (sender, mut reciever) = socket::channel(&src_net_interface);
        info!("send the icmp packet...");
        sender.sendto(send_packet);

        // パケットの送受信
        info!("receive the icmp packet...");
        while let Ok((_ret, _addr)) = reciever.recvfrom() {
            if !reciever.buf.is_empty() && Icmp::is_icmp_reply_packet(&reciever.buf) {
                info!("found an icmp reply packet...");
                let offset = ETHERNET_FRAME_LENGTH + IP_HEADER_LENGTH;
                return Some(IcmpFrame::from_bytes(&reciever.buf[offset..]));
            }
        }
        None
    }

    fn is_icmp_reply_packet(packet: &[u8]) -> bool {
        packet[23] == 0x01
    }
}

#[allow(dead_code)]
pub enum MessageType {
    Echo,
    EchoReply,
}

// ICMP ping (Echo or Echo Reply Message)
// cf: https://datatracker.ietf.org/doc/html/rfc792
#[derive(Debug)]
pub struct IcmpFrame {
    frame_type: u8,
    // ICMP ping の場合は常に0
    code: u8,
    checksum: u16,
    // identification と seq_num は、ping の reply が期待しているものかどうかを判定するために使われる
    // 送る側は、identification は同じ値を、seq_num は一定値増加させた値をセットする
    // ICMP ping を受け取った側は、そのまま同じ値で返す
    identification: u16,
    seq_num: u16,
}

impl IcmpFrame {
    fn new(frame_type: MessageType) -> IcmpFrame {
        let frame_type = match frame_type {
            MessageType::Echo => 8,
            MessageType::EchoReply => 0,
        };
        IcmpFrame {
            code: 0,           // 常に 0
            checksum: 0,       // 後でセットする
            identification: 0, // なんでも良いので、今回は 0
            seq_num: 0,        // 初期値はなんでも良いので、今回は 0
            frame_type,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> IcmpFrame {
        IcmpFrame {
            frame_type: bytes[0],
            code: bytes[1],
            checksum: u16::from_be_bytes([bytes[2], bytes[3]]),
            identification: u16::from_be_bytes([bytes[4], bytes[5]]),
            seq_num: u16::from_be_bytes([bytes[6], bytes[7]]),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(self.frame_type);
        bytes.push(self.code);
        bytes.extend(&self.checksum.to_be_bytes());
        bytes.extend(&self.identification.to_be_bytes());
        bytes.extend(&self.seq_num.to_be_bytes());
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
        bytes[2..4].copy_from_slice(&(0xFFFF - checksum as u16).to_be_bytes());
    }
}
