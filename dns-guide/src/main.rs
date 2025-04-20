use std::net::UdpSocket;

use buffer::BytePacketBuffer;
use header::ResultCode;
use packet::DnsPacket;
use question::{DnsQuestion, QueryType};

mod buffer;
mod header;
mod packet;
mod question;
mod record;
mod utils;

fn main() -> utils::Result<()> {
    // UDP ソケットを作成
    let socket = UdpSocket::bind(("0.0.0.0", 2053))?;

    loop {
        match handle_query(&socket) {
            Ok(_) => {}
            Err(e) => eprintln!("An error occurred: {}", e),
        }
    }
}

// DNS サーバーが受け取ったパケットを処理するハンドラー
fn handle_query(socket: &UdpSocket) -> utils::Result<()> {
    // DNS サーバーが受け取ったパケットを受信
    let mut req_buffer = BytePacketBuffer::new();
    let (_, src) = socket.recv_from(&mut req_buffer.buf)?;

    // 受信したパケットを解析
    let mut request = DnsPacket::from_buffer(&mut req_buffer)?;

    // 応答パケットを作成
    let mut packet = DnsPacket::new();
    packet.header.id = request.header.id;
    packet.header.recursion_desired = true;
    packet.header.recursion_available = true;
    packet.header.response = true;

    // Question が１つの場合を想定している
    if let Some(question) = request.questions.pop() {
        println!("Received query: {:?}", question);

        if let Ok(result) = lookup(&question.name, question.qtype) {
            packet.questions.push(question);
            packet.header.rescode = result.header.rescode;

            for rec in result.answers {
                println!("Answer: {:?}", rec);
                packet.answers.push(rec);
            }
            for rec in result.authorities {
                println!("Authority: {:?}", rec);
                packet.authorities.push(rec);
            }
            for rec in result.resources {
                println!("Resource: {:?}", rec);
                packet.resources.push(rec);
            }
        } else {
            packet.header.rescode = ResultCode::SERVFAIL;
        }
    } else {
        packet.header.rescode = ResultCode::FORMERR;
    }

    // 応答パケットの送信
    let mut res_buffer = BytePacketBuffer::new();
    packet.write(&mut res_buffer)?;
    let len = res_buffer.pos();
    let data = res_buffer.get_range(0, len)?;
    socket.send_to(data, src)?;

    Ok(())
}

fn lookup(qname: &str, qtype: QueryType) -> utils::Result<DnsPacket> {
    // Google's public DNS を使う
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

    // DNS の問い合わせパケットを送信
    let mut req_buffer = BytePacketBuffer::new();
    packet.write(&mut req_buffer)?;
    socket.send_to(&req_buffer.buf[0..req_buffer.pos], server)?;

    // DNS の応答パケットを受信
    let mut res_buffer = BytePacketBuffer::new();
    socket.recv_from(&mut res_buffer.buf)?;
    DnsPacket::from_buffer(&mut res_buffer)
}
