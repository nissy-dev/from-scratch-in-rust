use std::{os::fd::RawFd, path::PathBuf};

use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use structopt::StructOpt;

use crate::errors::ErrorCode;

#[derive(Debug, StructOpt)]
#[structopt(name = "crabcan", about = "A simple container in Rust.")]
pub struct Args {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Command to execute inside the container
    #[structopt(short, long)]
    pub command: String,

    /// User ID to create inside the container
    #[structopt(short, long)]
    pub uid: u32,

    /// Directory to mount as root of the container
    #[structopt(parse(from_os_str), short = "m", long = "mount")]
    pub mount_dir: PathBuf,

    /// Mount a directory inside the container
    #[structopt(parse(from_os_str), short = "a", long = "add")]
    pub add_paths: Vec<PathBuf>,
}

pub fn parse_args() -> Result<Args, ErrorCode> {
    let args = Args::from_args();
    if args.debug {
        setup_logging(log::LevelFilter::Debug);
    } else {
        setup_logging(log::LevelFilter::Info);
    }

    if !args.mount_dir.exists() || !args.mount_dir.is_dir() {
        return Err(ErrorCode::ArgumentInvalid("mount"));
    }

    if args.command.is_empty() {
        return Err(ErrorCode::ArgumentInvalid("command"));
    }

    Ok(args)
}

pub fn setup_logging(level: log::LevelFilter) {
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .filter(None, level)
        .init();
}

pub fn generate_socketpair() -> Result<(RawFd, RawFd), ErrorCode> {
    match socketpair(
        AddressFamily::Unix,
        SockType::SeqPacket,
        None,
        SockFlag::SOCK_CLOEXEC,
    ) {
        Ok(res) => Ok(res),
        Err(_) => Err(ErrorCode::SocketError(0)),
    }
}
