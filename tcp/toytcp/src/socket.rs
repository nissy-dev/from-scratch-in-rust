use anyhow::{Context, Result};
use core::fmt;
use std::{
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
};

use pnet::{
    packet::{ip::IpNextHeaderProtocols, util, Packet},
    transport::{self, TransportChannelType, TransportProtocol, TransportSender},
};

use crate::packet::TCPPacket;

const SOCKET_BUFFER_SIZE: usize = 4380;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SockID(pub Ipv4Addr, pub Ipv4Addr, pub u16, pub u16);

pub struct Socket {
    pub local_addr: Ipv4Addr,
    pub remote_addr: Ipv4Addr,
    pub local_port: u16,
    pub remote_port: u16,
    pub send_param: SendParam,
    pub recv_param: RecvParam,
    pub status: TcpStatus,
    pub sender: TransportSender,
}

#[derive(Clone, Debug)]
pub struct SendParam {
    pub initial_seq: u32,  // 最初の送信シーケンス番号 (SND.ISS)
    pub unpacked_seq: u32, // 送信後まだ ack されてないシーケンスの先頭 (SND.UNA)
    pub next: u32,         // 送信する seq の先頭 (SND.NXT)
    pub window: u16,       // 送信ウィンドウサイズ (SND.WND)
}

#[derive(Clone, Debug)]
pub struct RecvParam {
    pub initial_seq: u32, // 最初の受信シーケンス番号 (RCV.ISS)
    pub next: u32,        // 次に受信する seq の先頭 (RCV.NXT)
    pub tail: u32,        // ?
    pub window: u16,      // 受信ウィンドウサイズ (RCV.WND)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TcpStatus {
    Listen,
    SynSent,
    SynRcvd,
    Established,
    FinWait1,
    FinWait2,
    TimeWait,
    CloseWait,
    LastAck,
}

impl Display for TcpStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TcpStatus::Listen => write!(f, "LISTEN"),
            TcpStatus::SynSent => write!(f, "SYNSENT"),
            TcpStatus::SynRcvd => write!(f, "SYNRCVD"),
            TcpStatus::Established => write!(f, "ESTABLISHED"),
            TcpStatus::FinWait1 => write!(f, "FINWAIT1"),
            TcpStatus::FinWait2 => write!(f, "FINWAIT2"),
            TcpStatus::TimeWait => write!(f, "TIMEWAIT"),
            TcpStatus::CloseWait => write!(f, "CLOSEWAIT"),
            TcpStatus::LastAck => write!(f, "LASTACK"),
        }
    }
}

impl Socket {
    pub fn new(
        local_addr: Ipv4Addr,
        remote_addr: Ipv4Addr,
        local_port: u16,
        remote_port: u16,
        status: TcpStatus,
    ) -> Result<Self> {
        let (sender, _) = transport::transport_channel(
            65535,
            TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp)),
        )?;
        Ok(Self {
            local_addr,
            remote_addr,
            local_port,
            remote_port,
            send_param: SendParam {
                initial_seq: 0,
                unpacked_seq: 0,
                window: SOCKET_BUFFER_SIZE as u16,
                next: 0,
            },
            recv_param: RecvParam {
                initial_seq: 0,
                window: SOCKET_BUFFER_SIZE as u16,
                next: 0,
                tail: 0,
            },
            status,
            sender,
        })
    }

    pub fn send_tcp_packet(
        &mut self,
        seq: u32,
        ack: u32,
        flag: u8,
        payload: &[u8],
    ) -> Result<usize> {
        let mut tcp_packet = TCPPacket::new(payload.len());
        tcp_packet.set_src(self.local_port);
        tcp_packet.set_dest(self.remote_port);
        tcp_packet.set_seq(seq);
        tcp_packet.set_ack(ack);
        tcp_packet.set_data_offset(5); // オプションフィールドは利用しないので固定値を入れる
        tcp_packet.set_flag(flag);
        tcp_packet.set_window_size(self.recv_param.window);
        tcp_packet.set_payload(payload);
        tcp_packet.set_checksum(util::ipv4_checksum(
            &tcp_packet.packet(),
            0,
            &[],
            &self.local_addr,
            &self.remote_addr,
            IpNextHeaderProtocols::Tcp,
        ));
        let sent_size = self
            .sender
            .send_to(tcp_packet.clone(), IpAddr::V4(self.remote_addr))
            .context("failed to send packet")?;
        dbg!("sent", &tcp_packet);
        Ok(sent_size)
    }

    pub fn get_sock_id(&self) -> SockID {
        SockID(
            self.local_addr,
            self.remote_addr,
            self.local_port,
            self.remote_port,
        )
    }
}
