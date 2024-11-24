use crossbeam_channel::{bounded, Receiver, Sender};
use nix::unistd::{read, write};
use std::fs::OpenOptions;
use std::os::fd::AsFd;
use std::sync::Arc;
use std::thread;
use std::{fs::File, os::fd::AsRawFd};
use tracing::info;

const TUNSETIFF: u64 = 0x400454ca;
const IFF_TUN: u16 = 0x0001;
const IFF_NO_PI: u16 = 0x1000;

#[repr(C)]
struct IfReq {
    ifr_name: [u8; 16],
    ifr_flags: u16,
}

#[derive(Debug)]
pub struct Packet {
    data: Vec<u8>,
    length: usize,
}

type Channel = (Sender<Packet>, Receiver<Packet>);

#[derive(Debug, Clone)]
pub struct NetDevice {
    file: Arc<File>,
    incoming_queue: Arc<Channel>,
    outgoing_queue: Arc<Channel>,
}

impl NetDevice {
    pub fn new() -> NetDevice {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .expect("failed to open TUN device");

        // ifreq 構造体を初期化し、インターフェース名とフラグを設定
        let mut ifr = IfReq {
            ifr_name: [0u8; 16],
            ifr_flags: IFF_TUN | IFF_NO_PI,
        };
        let name_bytes = "tun0".as_bytes();
        if name_bytes.len() >= ifr.ifr_name.len() {
            panic!("interface name is too long");
        }
        ifr.ifr_name[..name_bytes.len()].copy_from_slice(name_bytes);

        // ioctl で TUN インターフェースを設定
        unsafe {
            nix::libc::ioctl(file.as_raw_fd(), TUNSETIFF, &ifr as *const _);
        }

        NetDevice {
            file: Arc::new(file),
            incoming_queue: Arc::new(bounded::<Packet>(10)),
            outgoing_queue: Arc::new(bounded::<Packet>(10)),
        }
    }

    pub fn bind(&self) {
        let read_file = self.file.clone();
        let incoming_queue = self.incoming_queue.clone();
        thread::spawn(move || loop {
            info!("read packet from TUN device...");
            let mut buffer = [0; 2048];
            let length =
                read(read_file.as_raw_fd(), &mut buffer).expect("failed to read from TUN device");
            let packet = Packet {
                data: buffer[..length].to_vec(),
                length,
            };
            let (in_sender, _) = incoming_queue.as_ref();
            in_sender.send(packet).expect("failed to send packet");
        });

        let write_file = self.file.clone();
        let outgoing_queue = self.outgoing_queue.clone();
        thread::spawn(move || loop {
            let (_, out_receiver) = outgoing_queue.as_ref();
            let packet = out_receiver.recv().expect("failed to receive packet");
            info!("write packet to TUN device");
            write(write_file.as_fd(), &packet.data).expect("failed to write to TUN device");
        });
    }

    pub fn read(&self) -> Packet {
        let (_, receiver) = self.incoming_queue.as_ref();
        receiver.recv().expect("failed to receive packet")
    }

    pub fn write(&self, packet: Packet) {
        let (sender, _) = self.outgoing_queue.as_ref();
        sender.send(packet).expect("failed to send packet");
    }
}
