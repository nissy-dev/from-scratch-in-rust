use std::os::fd::RawFd;

use nix::sys::socket;

pub struct SocketCommunication {
    socket_fd: Option<RawFd>,
}

impl SocketCommunication {
    fn new() -> Self {
        let fd = socket();
    }
}
