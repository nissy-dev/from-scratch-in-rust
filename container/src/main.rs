#[macro_use]
extern crate scan_fmt;

use std::process::exit;

use crate::errors::exit_with_return_code;

mod capabilities;
mod check_linux_version;
mod child;
mod cli;
mod config;
mod container;
mod errors;
mod hostname;
mod ipc;
mod mount;
mod namespaces;

fn main() {
    match cli::parse_args() {
        Ok(args) => {
            log::info!("{:?}", args);
            exit_with_return_code(container::start(args));
        }
        Err(e) => {
            log::error!("Error while parsing arguments:\n\t{}", e);
            exit(e.get_return_code())
        }
    }
}
