use std::{
    fs::{create_dir_all, remove_dir},
    path::PathBuf,
};

use nix::{
    mount::{mount, umount2, MntFlags, MsFlags},
    unistd::{chdir, pivot_root},
};
use rand::Rng;

use crate::errors::ErrorCode;

pub fn set_mountpoint(mount_dir: &PathBuf) -> Result<(), ErrorCode> {
    log::debug!("Setting mount points ...");

    // まずは、新しい root となるディレクトリを作成する
    let new_root = PathBuf::from(format!("/tmp/crabcan.{}", random_string(12)));
    log::debug!(
        "Mounting temp directory {}",
        new_root.as_path().to_str().unwrap()
    );
    create_directory(&new_root)?;
    // 引数に渡されたディレクトリをマウントする
    mount_directory(
        Some(mount_dir),
        &new_root,
        vec![MsFlags::MS_BIND, MsFlags::MS_PRIVATE],
    )?;
    // root ディレクトリを変更する (元の root は /oldroot.xxxx になる)
    log::debug!("Pivoting root");
    let old_root_tail = format!("oldroot.{}", random_string(6));
    let put_old = new_root.join(PathBuf::from(&old_root_tail));
    create_directory(&put_old)?;
    if let Err(_) = pivot_root(&new_root, &put_old) {
        return Err(ErrorCode::MountsError(4));
    }
    // 古い root (oldrot.xxxx) をアンマウントする
    log::debug!("Unmounting old root");
    let old_root = PathBuf::from(format!("/{}", &old_root_tail));
    // root ディレクトリに移動して、unmount のディレクトリにいないことを保証する
    if let Err(_) = chdir(&PathBuf::from("/")) {
        return Err(ErrorCode::MountsError(5));
    }
    unmount_path(&old_root)?;
    delete_directory(&old_root)?;
    Ok(())
}

pub fn clean_mounts(mount_dir: &PathBuf) -> Result<(), ErrorCode> {
    Ok(())
}

fn mount_directory(
    path: Option<&PathBuf>,
    mount_point: &PathBuf,
    flags: Vec<MsFlags>,
) -> Result<(), ErrorCode> {
    let mut ms_flags = MsFlags::empty();
    for flag in flags.iter() {
        ms_flags.insert(*flag);
    }

    match mount::<PathBuf, PathBuf, PathBuf, PathBuf>(path, mount_point, None, ms_flags, None) {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(p) = path {
                log::error!(
                    "Cannot mount {} to {}: {}",
                    p.to_str().unwrap(),
                    mount_point.to_str().unwrap(),
                    e
                );
            } else {
                log::error!("Cannot remount {}: {}", mount_point.to_str().unwrap(), e);
            }
            Err(ErrorCode::MountsError(3))
        }
    }
}

fn create_directory(path: &PathBuf) -> Result<(), ErrorCode> {
    match create_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Cannot create directory {}: {}", path.to_str().unwrap(), e);
            Err(ErrorCode::MountsError(2))
        }
    }
}

fn random_string(n: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                          abcdefghijklmnopqrstuvwxyz\
                          0123456789";
    let mut rng = rand::thread_rng();

    let name: String = (0..n)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    name
}

fn unmount_path(path: &PathBuf) -> Result<(), ErrorCode> {
    match umount2(path, MntFlags::MNT_DETACH) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Unable to umount {}: {}", path.to_str().unwrap(), e);
            Err(ErrorCode::MountsError(0))
        }
    }
}

fn delete_directory(path: &PathBuf) -> Result<(), ErrorCode> {
    match remove_dir(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!(
                "Unable to delete directory {}: {}",
                path.to_str().unwrap(),
                e
            );
            Err(ErrorCode::MountsError(1))
        }
    }
}
