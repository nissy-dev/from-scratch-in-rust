use nix::sys::utsname::uname;

use crate::errors::ErrorCode;

pub const MINIMAL_KERNEL_VERSION: f32 = 4.8;

pub fn check_linux_version() -> Result<(), ErrorCode> {
    let host = uname();
    log::debug!("Linux release: {}", host.release());

    if let Ok(version) = scan_fmt!(host.release(), "{f}.{}", f32) {
        if version < MINIMAL_KERNEL_VERSION {
            return Err(ErrorCode::NotSupported(0));
        }
    } else {
        return Err(ErrorCode::ContainerError(0));
    }

    // if host.machine() != "x86_64" {
    //     return Err(ErrorCode::NotSupported(1));
    // }

    Ok(())
}
