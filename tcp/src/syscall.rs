use std::{mem, ptr};

use libc::{c_void, sockaddr_ll};

pub fn socket(domain: i32, ty: i32, protocol: i32) -> i32 {
    let ret = unsafe { libc::socket(domain, ty, protocol) };
    if ret < 0 {
        eprintln!("failed to create socket");
        std::process::exit(1);
    }
    ret
}

pub fn sendto(fd: i32, packet: &[u8], addr: &sockaddr_ll) -> isize {
    let ret = unsafe {
        libc::sendto(
            fd,
            packet.as_ptr() as *const c_void,
            packet.len(),
            0,
            addr as *const _ as *const libc::sockaddr,
            mem::size_of::<sockaddr_ll>() as u32,
        )
    };
    if ret < 0 {
        eprintln!("failed to send socket");
        std::process::exit(1);
    }
    ret
}

pub fn recvfrom(fd: i32, packet: &mut [u8]) -> isize {
    let ret = unsafe {
        libc::recvfrom(
            fd,
            packet.as_mut_ptr() as *mut c_void,
            packet.len(),
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if ret < 0 {
        eprintln!("failed to read socket");
        std::process::exit(1);
    }
    ret
}
