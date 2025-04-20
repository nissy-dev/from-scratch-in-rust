use std::net::{Ipv4Addr, UdpSocket};

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

        if let Ok(result) = recursive_lookup(&question.name, question.qtype) {
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

fn recursive_lookup(qname: &str, qtype: QueryType) -> utils::Result<DnsPacket> {
    // root NS server は 13 個あり、その１つのアドレスを使う
    // ref: https://www.internic.net/domain/named.root
    let mut ns = "198.41.0.4".parse::<Ipv4Addr>().unwrap();

    loop {
        println!("attempting lookup of {:?} {} with ns {}", qtype, qname, ns);

        let ns_copy = ns;
        // dns server は通常 53 番ポートで待ち受けている
        let server = (ns_copy, 53);
        // DNS サーバーに問い合わせて、応答を受け取る
        let response = lookup(qname, qtype, server)?;

        // Answer が存在する場合は、再帰問い合わせの最後なので結果を返す
        if !response.answers.is_empty() && response.header.rescode == ResultCode::NOERROR {
            return Ok(response);
        }

        // レスポンスコードが NXDOMAIN の場合は、存在しないドメイン名なのでそのまま response を返す
        if response.header.rescode == ResultCode::NXDOMAIN {
            return Ok(response);
        }

        // 応答から次の NS server の ipv4 アドレスが分かった場合は、その値を ns の変数にセットしてループを続ける
        if let Some(new_ns) = response.get_resolved_ns(qname) {
            ns = new_ns;
            continue;
        }

        // NS server の ipv4 アドレスがわからなかった場合
        // NS server のホスト名を取得できるか確認する
        let new_ns_name = match response.get_unresolved_ns(qname) {
            Some(x) => x,
            // ここは期待してないので、そのまま response を返す
            None => return Ok(response),
        };
        // 再帰問い合わせを行って ipv4 アドレスを取得する
        let recursive_response = recursive_lookup(&new_ns_name, QueryType::A)?;
        if let Some(new_ns) = recursive_response.get_random_a() {
            ns = new_ns;
        } else {
            // もし NS server の ipv4 アドレスが取得できなかった場合は、そのまま response を返す
            return Ok(response);
        }
    }
}

fn lookup(qname: &str, qtype: QueryType, server: (Ipv4Addr, u16)) -> utils::Result<DnsPacket> {
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
