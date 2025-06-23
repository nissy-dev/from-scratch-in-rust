use anyhow::Result;
use std::{fs::File, io::Write, os::fd::FromRawFd};

use super::{store::Store, value::Value};

#[derive(Default)]
pub struct WasiSnapShotPreview1 {
    pub file_table: Vec<Box<File>>,
}

impl WasiSnapShotPreview1 {
    pub fn new() -> Self {
        unsafe {
            Self {
                // ３つのテーブルは stdin, stdout, stderr に対応している
                file_table: vec![
                    Box::new(File::from_raw_fd(0)),
                    Box::new(File::from_raw_fd(1)),
                    Box::new(File::from_raw_fd(2)),
                ],
            }
        }
    }

    pub fn invoke(
        &mut self,
        store: &mut Store,
        func: &str,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        match func {
            "fd_write" => self.fd_write(store, args),
            _ => unimplemented!("{}", func),
        }
    }

    fn fd_write(&mut self, store: &mut Store, args: Vec<Value>) -> Result<Option<Value>> {
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();

        let fd = args[0];
        let mut iovs = args[1] as usize;
        let iovs_len = args[2];
        let rp = args[3] as usize;
        let file = self
            .file_table
            .get_mut(fd as usize)
            .ok_or_else(|| anyhow::anyhow!("invalid file descriptor: {}", fd))?;
        let memory = store
            .memories
            .get_mut(0)
            .ok_or(anyhow::anyhow!("not found memory"))?;

        let mut nwritten = 0;
        for _ in 0..iovs_len {
            // なんでここは 4 byte ずらしているんだろう？
            // → メモリは4バイトでアライメントされているため
            let start = memory_read(&memory.data, iovs)? as usize;
            iovs += 4;

            let len = memory_read(&memory.data, iovs)? as usize;
            iovs += 4;

            let end = start + len;
            nwritten += file.write(&memory.data[start..end])?;
        }

        memory_write(&mut memory.data, rp, &nwritten.to_le_bytes())?;

        Ok(Some(0.into()))
    }
}

fn memory_read(buf: &[u8], start: usize) -> Result<i32> {
    let end = start + 4;
    Ok(<i32>::from_le_bytes(buf[start..end].try_into()?))
}

fn memory_write(buf: &mut Vec<u8>, start: usize, data: &[u8]) -> Result<()> {
    let end = start + data.len();
    buf[start..end].copy_from_slice(data);
    Ok(())
}
