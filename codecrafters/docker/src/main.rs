use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use libc::{c_char, c_int};
use reqwest::Client;
use serde::Deserialize;
use tar::Archive;
use tempfile::{tempdir, TempDir};

use std::{
    env::{self, set_current_dir},
    ffi::CString,
    fs::{copy, create_dir_all, set_permissions, File, Permissions},
    io::{stderr, stdout, Write},
    os::unix::prelude::PermissionsExt,
    process::{exit, Command},
};

const EXEC_MODE: u32 = 0o777; // Read/write/execute for owner and group, read/execute for others
const RW_MODE: u32 = 0o666; // Read/write for owner and group, read for others
const CLONE_NEWPID: c_int = 0x20000000;

extern "C" {
    fn chroot(name: *const c_char) -> c_int;
    fn unshare(flags: c_int);
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FsLayer {
    blob_sum: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestResponse {
    name: String,
    fs_layers: Vec<FsLayer>,
}

fn create_dev_null(temp_dir: &TempDir) -> Result<()> {
    let dev_dir_path = temp_dir.path().join("dev");
    create_dir_all(&dev_dir_path)?;
    set_permissions(&dev_dir_path, Permissions::from_mode(RW_MODE))?;
    let null_file_path = temp_dir.path().join("dev/null");
    File::create(null_file_path)?.set_permissions(Permissions::from_mode(RW_MODE))?;
    Ok(())
}

fn copy_executable_binary(command: &str, temp_dir: &TempDir) -> Result<()> {
    let dist_path = temp_dir.path().join(command.trim_start_matches("/"));
    create_dir_all(&dist_path.parent().unwrap())?;
    copy(command, &dist_path)?;
    set_permissions(&dist_path, Permissions::from_mode(EXEC_MODE))?;
    Ok(())
}

// Usage: your_docker.sh run <image> <command> <arg1> <arg2> ...
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let image = if args[2].contains(":") {
        String::from(&args[2])
    } else {
        format!("{}:latest", &args[2])
    };
    let command = &args[3];
    let command_args = &args[4..];

    // create temporary directory and /dev/null
    let dir = tempdir()?;
    create_dev_null(&dir)?;

    // copy executable binary
    copy_executable_binary(command, &dir)?;

    // fetch docker image
    let image_metadata = image.split(':').collect::<Vec<_>>();
    let client = Client::new();
    let auth_endpoint = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/{}:pull",
        image_metadata[0]
    );
    let AuthResponse { token } = client
        .get(&auth_endpoint)
        .send()
        .await?
        .json::<AuthResponse>()
        .await?;

    let image_manifest_endpoint = format!(
        "https://registry.hub.docker.com/v2/library/{}/manifests/{}",
        &image_metadata[0], &image_metadata[1],
    );
    let ManifestResponse { name, fs_layers } = client
        .get(&image_manifest_endpoint)
        .header("Authorization", format!("Bearer {}", &token))
        .send()
        .await?
        .json::<ManifestResponse>()
        .await?;

    for layer in fs_layers {
        let layer_endpoint = format!(
            "https://registry.hub.docker.com/v2/{}/blobs/{}",
            &name, &layer.blob_sum,
        );
        // layer data is tar.gzip format. This information is included in the manifest.
        // see: https://docs.docker.com/registry/spec/manifest-v2-2/
        let layer_blob = client
            .get(&layer_endpoint)
            .header("Authorization", format!("Bearer {}", &token))
            .send()
            .await?
            .bytes()
            .await?;

        // write layer data to gzipped tar file and extract it
        let tar = GzDecoder::new(&*layer_blob);
        let mut archive = Archive::new(tar);
        archive.unpack(&dir)?;
    }

    // chroot to temporary directory and set current directory to /
    let dir_path = CString::new(dir.path().to_str().unwrap())?;
    unsafe {
        chroot(dir_path.as_ptr());
    }
    set_current_dir("/")?;

    // unshare PID namespace
    unsafe {
        unshare(CLONE_NEWPID);
    }

    let output = Command::new(command)
        .args(command_args)
        .output()
        .with_context(|| {
            format!(
                "Tried to run '{}' with arguments {:?}",
                command, command_args
            )
        })?;

    stdout().write_all(&output.stdout)?;
    stderr().write_all(&output.stderr)?;

    exit(output.status.code().unwrap_or(1));
}
