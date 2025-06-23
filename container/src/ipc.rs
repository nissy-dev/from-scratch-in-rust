use std::os::fd::RawFd;

use nix::sys::socket::{recv, send, MsgFlags};

use crate::errors::ErrorCode;

pub fn send_boolean(fd: RawFd, value: bool) -> Result<(), ErrorCode> {
    let data: [u8; 1] = [value.into()];
    if let Err(e) = send(fd, &data, MsgFlags::empty()) {
        log::error!("Cannot send boolean through socket: {:?}", e);
        return Err(ErrorCode::SocketError(1));
    }
    Ok(())
}

pub fn recv_boolean(fd: RawFd) -> Result<bool, ErrorCode> {
    let mut data: [u8; 1] = [0];
    if let Err(e) = recv(fd, &mut data, MsgFlags::empty()) {
        log::error!("Cannot receive boolean from socket: {:?}", e);
        return Err(ErrorCode::SocketError(2));
    }
    Ok(data[0] == 1)
}
