use std::{ffi::CString, os::fd::RawFd, path::PathBuf};

use crate::{cli::generate_socketpair, errors::ErrorCode, hostname::generate_hostname};

#[derive(Clone)]
pub struct ContainerOpts {
    pub path: CString,
    pub argv: Vec<CString>,
    pub uid: u32,
    pub mount_dir: PathBuf,
    pub fd: RawFd,
    pub hostname: String,
}

impl ContainerOpts {
    pub fn new(
        command: String,
        uid: u32,
        mount_dir: PathBuf,
    ) -> Result<(Self, (RawFd, RawFd)), ErrorCode> {
        let argv = command
            .split_ascii_whitespace()
            .map(|s| CString::new(s).expect("Cannot read arg"))
            .collect::<Vec<_>>();
        let path = argv[0].clone();
        let sockets = generate_socketpair()?;

        Ok((
            Self {
                path,
                argv,
                uid,
                mount_dir,
                fd: sockets.1.clone(),
                hostname: generate_hostname()?,
            },
            sockets,
        ))
    }
}
