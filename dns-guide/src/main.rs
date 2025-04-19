use std::net::UdpSocket;

use buffer::BytePacketBuffer;
use packet::DnsPacket;
use question::{DnsQuestion, QueryType};

mod buffer;
mod header;
mod packet;
mod question;
mod record;
mod utils;

fn main() -> utils::Result<()> {
    let qname = "www.yahoo.com";
    let qtype = QueryType::A;

    // Google Public DNS の IP アドレス
    let server = ("8.8.8.8", 53);
    // UDP で DNS を問い合わせるためのソケットを作成
    let socket = UdpSocket::bind(("0.0.0.0", 43210))?;

    // DNS の問い合わせパケットを作成
    let mut packet = DnsPacket::new();
    packet.header.id = 6666;
    packet.header.questions = 1;
    packet.header.recursion_desired = true;
    packet
        .questions
        .push(DnsQuestion::new(qname.to_string(), qtype));
    // buffer に書き込み
    let mut req_buffer = BytePacketBuffer::new();
    packet.write(&mut req_buffer)?;

    // DNS の問い合わせパケットを送信
    socket.send_to(&req_buffer.buf[0..req_buffer.pos], server)?;

    // DNS の応答パケットを受信
    let mut res_buffer = BytePacketBuffer::new();
    socket.recv_from(&mut res_buffer.buf)?;

    // 応答パケットを解析
    let res_packet = DnsPacket::from_buffer(&mut res_buffer)?;
    println!("{:#?}", res_packet.header);

    println!("Questions:");
    for q in res_packet.questions {
        println!("{:#?}", q);
    }
    println!("Answers:");
    for rec in res_packet.answers {
        println!("{:#?}", rec);
    }
    println!("Authorities:");
    for rec in res_packet.authorities {
        println!("{:#?}", rec);
    }
    for rec in res_packet.resources {
        println!("{:#?}", rec);
    }

    Ok(())
}
