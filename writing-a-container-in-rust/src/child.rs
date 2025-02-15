use nix::sched::clone;
use nix::sched::CloneFlags;
use nix::sys::signal::Signal;
use nix::unistd::close;
use nix::unistd::execve;
use nix::unistd::Pid;
use std::ffi::CString;

use crate::capabilities::set_capabilities;
use crate::hostname::set_container_hostname;
use crate::mount::set_mountpoint;
use crate::namespaces::userns;
use crate::syscalls::set_syscalls;
use crate::{config::ContainerOpts, errors::ErrorCode};

const STACK_SIZE: usize = 1024 * 1024;

fn set_container_configurations(config: &ContainerOpts) -> Result<(), ErrorCode> {
    set_container_hostname(&config.hostname)?;
    set_mountpoint(&config.mount_dir, &config.add_paths)?;
    userns(config.fd, config.uid)?;
    set_capabilities()?;
    set_syscalls()?;
    Ok(())
}

fn child(config: ContainerOpts) -> isize {
    match set_container_configurations(&config) {
        Ok(_) => log::info!("Container set up successfully"),
        Err(e) => {
            println!("Error while configuring container: {:?}", e);
            return -1;
        }
    }
    log::info!(
        "Starting container with command {} and args {:?}",
        config.path.to_str().unwrap(),
        config.argv
    );

    if let Err(_) = close(config.fd) {
        log::error!("Error while closing socket ...");
        return -1;
    }

    log::info!(
        "Starting container with command {} and args {:?}",
        config.path.to_str().unwrap(),
        config.argv
    );
    let retcode = match execve::<CString, CString>(&config.path, &config.argv, &[]) {
        Ok(_) => 0,
        Err(e) => {
            log::error!("Error while trying to perform execve: {:?}", e);
            -1
        }
    };
    retcode
}

pub fn generate_child_process(config: ContainerOpts) -> Result<Pid, ErrorCode> {
    let mut tmp_stack: [u8; STACK_SIZE] = [0; STACK_SIZE];
    let mut flags = CloneFlags::empty();
    // いろんな namespace を新しく作成するフラグを設定する
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWCGROUP);
    flags.insert(CloneFlags::CLONE_NEWPID);
    flags.insert(CloneFlags::CLONE_NEWIPC);
    flags.insert(CloneFlags::CLONE_NEWNET);
    flags.insert(CloneFlags::CLONE_NEWUTS);
    match clone(
        Box::new(|| child(config.clone())),
        &mut tmp_stack,
        flags,
        Some(Signal::SIGCHLD as i32),
    ) {
        Ok(pid) => Ok(pid),
        Err(e) => {
            println!("Error while creating child process: {:?}", e);
            Err(ErrorCode::ChildProcessError(1))
        }
    }
}
