use crate::checksum;
use std::net::Ipv4Addr;

// 基本は 20 byte だが、オプションフィールドがある場合はそれが追加される
pub const IP_HEADER_LENGTH: usize = 20;

pub enum Protocol {
    IP,
    UDP,
}

#[derive(Debug, Clone, Copy)]
pub struct IpHeader {
    version: u8,
    // Internet Header Length (IHL)：ヘッダの長さ
    ihl: u8,
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
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
}

impl IpHeader {
    pub fn new(
        src_ip: Ipv4Addr,
        dst_ip: Ipv4Addr,
        total_length: u16,
        protocol: Protocol,
    ) -> IpHeader {
        let protocol = match protocol {
            Protocol::IP => 0x01,
            Protocol::UDP => 0x11,
        };
        IpHeader {
            version: 4,                        // 常に 4
            ihl: (IP_HEADER_LENGTH / 4) as u8, // 32 ビット単位で表現するため 4 で割る
            tos: 0,                            // 優先度が一番低い 0 を指定
            identification: 0,                 // フラグメント化しないので 0
            flags: 2,                          // フラグメント化を許可しない (010)
            fragment_offset: 0,                // フラグメント化しないので 0
            ttl: 64,                           // 64, 128, 255 などを指定、今回は 64
            checksum: 0,                       // 後でセットする
            src_ip,
            dst_ip,
            total_length,
            protocol,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let version_and_ihl = (self.version << 4) | self.ihl;
        let flags_and_fragment_offset =
            ((self.flags as u16) << 13) | (self.fragment_offset & 0x1FFF);
        bytes.push(version_and_ihl);
        bytes.push(self.tos);
        bytes.extend(&self.total_length.to_be_bytes());
        bytes.extend(&self.identification.to_be_bytes());
        bytes.extend(&flags_and_fragment_offset.to_be_bytes());
        bytes.push(self.ttl);
        bytes.push(self.protocol);
        bytes.extend(&self.checksum.to_be_bytes());
        bytes.extend(&self.src_ip.octets());
        bytes.extend(&self.dst_ip.octets());
        // checksum を計算して、再度セットする
        self.set_checksum(&mut bytes);
        bytes
    }

    fn set_checksum(&self, bytes: &mut [u8]) {
        let checksum = checksum::checksum(bytes);
        bytes[10..12].copy_from_slice(&checksum.to_be_bytes());
    }
}
