use std::{os::fd::RawFd, path::PathBuf};

use nix::{
    sys::wait::waitpid,
    unistd::{close, Pid},
};

use crate::{
    check_linux_version::check_linux_version,
    child::generate_child_process,
    cli::Args,
    config::ContainerOpts,
    errors::ErrorCode,
    mount::clean_mounts,
    namespaces::handle_child_uid_map,
    resources::{clean_cgroups, restrict_resources},
};

pub struct Container {
    config: ContainerOpts,
    sockets: (RawFd, RawFd),
    child_pid: Option<Pid>,
}

impl Container {
    pub fn new(args: Args) -> Result<Self, ErrorCode> {
        let mut add_paths = vec![];
        for ap_pair in args.add_paths.iter() {
            let mut pair = ap_pair.to_str().unwrap().split(':');
            let from_path = PathBuf::from(pair.next().unwrap())
                .canonicalize()
                .expect("Cannot canonicalize path")
                .to_path_buf();
            let mnt_path = PathBuf::from(pair.next().unwrap())
                .strip_prefix("/")
                .expect("Cannot strip prefix from path")
                .to_path_buf();
            add_paths.push((from_path, mnt_path));
        }
        let (config, sockets) =
            ContainerOpts::new(args.command, args.uid, args.mount_dir, add_paths)?;
        Ok(Self {
            config,
            sockets,
            child_pid: None,
        })
    }

    pub fn create(&mut self) -> Result<(), ErrorCode> {
        let pid = generate_child_process(self.config.clone())?;
        restrict_resources(&self.config.hostname, pid)?;
        self.child_pid = Some(pid);
        handle_child_uid_map(pid, self.sockets.0)?;
        log::debug!("Creation finished");
        Ok(())
    }

    pub fn clean_exit(&mut self) -> Result<(), ErrorCode> {
        log::debug!("Cleaning container");

        if let Err(e) = close(self.sockets.0) {
            log::error!("Unable to close write socket: {:?}", e);
            return Err(ErrorCode::SocketError(3));
        }
        if let Err(e) = close(self.sockets.1) {
            log::error!("Unable to close read socket: {:?}", e);
            return Err(ErrorCode::SocketError(4));
        }

        clean_mounts(&self.config.mount_dir)?;

        if let Err(e) = clean_cgroups(&self.config.hostname) {
            log::error!("Cgroups cleaning failed: {}", e);
            return Err(e);
        }

        Ok(())
    }
}

pub fn start(args: Args) -> Result<(), ErrorCode> {
    check_linux_version()?;
    let mut container = Container::new(args)?;
    if let Err(e) = container.create() {
        container.clean_exit()?;
        log::error!("Error while creating container: {:?}", e);
        return Err(e);
    }

    log::debug!("Container child PID: {:?}", container.child_pid);
    wait_child(container.child_pid)?;

    log::debug!("Finished, cleaning & exit");
    container.clean_exit()
}

pub fn wait_child(pid: Option<Pid>) -> Result<(), ErrorCode> {
    if let Some(child_pid) = pid {
        log::debug!("Waiting for child (pid {}) to finish", child_pid);
        if let Err(e) = waitpid(child_pid, None) {
            log::error!("Error while waiting for pid to finish: {:?}", e);
            return Err(ErrorCode::ContainerError(1));
        }
    }

    Ok(())
}
