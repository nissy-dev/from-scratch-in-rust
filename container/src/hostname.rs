use nix::unistd::sethostname;
use rand::{seq::SliceRandom, Rng};

use crate::errors::ErrorCode;

const HOSTNAME_NAMES: [&'static str; 8] = [
    "cat", "world", "coffee", "girl", "man", "book", "pinguin", "moon",
];

const HOSTNAME_ADJ: [&'static str; 16] = [
    "blue",
    "red",
    "green",
    "yellow",
    "big",
    "small",
    "tall",
    "thin",
    "round",
    "square",
    "triangular",
    "weird",
    "noisy",
    "silent",
    "soft",
    "irregular",
];

pub fn generate_hostname() -> Result<String, ErrorCode> {
    let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
    let num = rng.gen::<u8>();
    let name = HOSTNAME_NAMES.choose(&mut rng).ok_or(ErrorCode::RngError)?;
    let adj = HOSTNAME_ADJ.choose(&mut rng).ok_or(ErrorCode::RngError)?;
    Ok(format!("{}-{}-{}", adj, name, num))
}

pub fn set_container_hostname(hostname: &str) -> Result<(), ErrorCode> {
    match sethostname(hostname) {
        Ok(_) => {
            log::debug!("Container hostname is now {}", hostname);
            Ok(())
        }
        Err(_) => {
            log::error!("Cannot set hostname {} for container", hostname);
            Err(ErrorCode::HostnameError(0))
        }
    }
}
