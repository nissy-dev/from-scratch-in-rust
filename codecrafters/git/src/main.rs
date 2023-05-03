use anyhow::Result;
use flate2::read::{ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use sha1::{Digest, Sha1};
use std::{env, fs, io::Read, io::Write};

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();

    match args[1].as_str() {
        "init" => {
            fs::create_dir(".git")?;
            fs::create_dir(".git/objects")?;
            fs::create_dir(".git/refs")?;
            fs::write(".git/HEAD", "ref: refs/heads/master\n")?;
            println!("Initialized git directory");

            Ok(())
        }
        "cat-file" => {
            if !(args.len() == 4 && &args[2] == "-p") {
                return Err(anyhow::anyhow!(
                    "Invalid arguments.\nusage: git cat-file -p <hash>"
                ));
            }

            let hash = &args[3];
            let data = fs::read(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))?;
            let mut decoder = ZlibDecoder::new(&*data);
            let mut raw_content = String::new();
            decoder.read_to_string(&mut raw_content)?;

            if let Some((_, content)) = raw_content.split_once("\0") {
                print!("{}", &content);
            } else {
                return Err(anyhow::anyhow!("Invalid blob object"));
            }

            Ok(())
        }
        "hash-object" => {
            if !(args.len() == 4 && &args[2] == "-w") {
                return Err(anyhow::anyhow!(
                    "Invalid arguments.\nusage: git hash-object -w <file>"
                ));
            }

            let file_content = fs::read_to_string(&args[3])?;
            let content = format!("blob {}\0{}", file_content.chars().count(), file_content);
            let hash = hex::encode(Sha1::digest(&content.as_bytes()));
            println!("{}", hash);

            let mut zlib_content = Vec::new();
            let mut encoder = ZlibEncoder::new(content.as_bytes(), Compression::fast());
            encoder.read_to_end(&mut zlib_content)?;

            fs::create_dir(format!(".git/objects/{}", &hash[..2]))?;
            let mut blob_file =
                fs::File::create(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))?;
            blob_file.write_all(&zlib_content)?;

            Ok(())
        }
        _ => Err(anyhow::anyhow!(format!("unknown command: {}", &args[1]))),
    }
}
