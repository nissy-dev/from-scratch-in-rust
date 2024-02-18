use std::{fs::File, io::Write, os::fd::RawFd};

use nix::{
    sched::{unshare, CloneFlags},
    unistd::{setgroups, setresgid, setresuid, Gid, Pid, Uid},
};

use crate::{
    errors::ErrorCode,
    ipc::{recv_boolean, send_boolean},
};

const USERNS_OFFSET: u64 = 10000;
const USERNS_COUNT: u64 = 2000;

// child process から呼び出される
pub fn userns(fd: RawFd, uid: u32) -> Result<(), ErrorCode> {
    log::debug!("Setting up user namespace with UID {}", uid);
    let has_userns = match unshare(CloneFlags::CLONE_NEWUSER) {
        Ok(_) => true,
        Err(_) => false,
    };
    send_boolean(fd, has_userns)?;

    if recv_boolean(fd)? {
        return Err(ErrorCode::NamespacesError(0));
    }

    if has_userns {
        log::info!("User namespaces set up");
    } else {
        log::info!("User namespaces not supported, continuing...");
    }

    log::debug!("Switching to uid {} / gid {}...", uid, uid);

    let gid = Gid::from_raw(uid);
    let uid = Uid::from_raw(uid);

    if let Err(_) = setgroups(&[gid]) {
        return Err(ErrorCode::NamespacesError(1));
    }

    if let Err(_) = setresgid(gid, gid, gid) {
        return Err(ErrorCode::NamespacesError(2));
    }

    if let Err(_) = setresuid(uid, uid, uid) {
        return Err(ErrorCode::NamespacesError(3));
    }
    Ok(())
}

// parent process から呼び出される
pub fn handle_child_uid_map(pid: Pid, fd: RawFd) -> Result<(), ErrorCode> {
    if recv_boolean(fd)? {
        if let Ok(mut uid_map) = File::create(format!("/proc/{}/uid_map", pid.as_raw())) {
            if let Err(_) =
                uid_map.write_all(format!("0 {} {}", USERNS_OFFSET, USERNS_COUNT).as_bytes())
            {
                return Err(ErrorCode::NamespacesError(4));
            }
        } else {
            return Err(ErrorCode::NamespacesError(5));
        }

        if let Ok(mut gid_map) = File::create(format!("/proc/{}/gid_map", pid.as_raw())) {
            if let Err(_) =
                gid_map.write_all(format!("0 {} {}", USERNS_OFFSET, USERNS_COUNT).as_bytes())
            {
                return Err(ErrorCode::NamespacesError(6));
            }
        } else {
            return Err(ErrorCode::NamespacesError(7));
        }
    } else {
        log::info!("No user namespace set up from child process");
    }

    log::debug!("Child UID/GID map done, sending signal to child to continue...");
    send_boolean(fd, false)
}
