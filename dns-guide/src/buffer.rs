use crate::utils::Result;

pub struct BytePacketBuffer {
    /// DNS の UDP packet の長さは基本 512 byte
    pub buf: [u8; 512],
    pub pos: usize,
}

impl BytePacketBuffer {
    pub fn new() -> BytePacketBuffer {
        BytePacketBuffer {
            buf: [0; 512],
            pos: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn step(&mut self, steps: usize) -> Result<()> {
        self.pos += steps;

        Ok(())
    }

    fn seek(&mut self, pos: usize) -> Result<()> {
        self.pos = pos;

        Ok(())
    }

    fn read(&mut self) -> Result<u8> {
        if self.pos >= 512 {
            return Err("End of buffer".into());
        }
        let res = self.buf[self.pos];
        self.pos += 1;

        Ok(res)
    }

    fn get(&mut self, pos: usize) -> Result<u8> {
        if pos >= 512 {
            return Err("End of buffer".into());
        }
        Ok(self.buf[pos])
    }

    pub fn get_range(&mut self, start: usize, len: usize) -> Result<&[u8]> {
        if start + len >= 512 {
            return Err("End of buffer".into());
        }
        Ok(&self.buf[start..start + len as usize])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let res = ((self.read()? as u16) << 8) | (self.read()? as u16);

        Ok(res)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let res = ((self.read()? as u32) << 24)
            | ((self.read()? as u32) << 16)
            | ((self.read()? as u32) << 8)
            | ((self.read()? as u32) << 0);

        Ok(res)
    }

    /// Read a name field from Question and Record objects
    pub fn read_qname(&mut self, outstr: &mut String) -> Result<()> {
        // 基本的な name filed は、以下のように dot 区切りで文字列長と文字列が表現されている
        //                     query name              type   class
        //        -----------------------------------  -----  -----
        // HEX    06 67 6f 6f 67 6c 65 03 63 6f 6d 00  00 01  00 01
        // ASCII     g  o  o  g  l  e     c  o  m
        // DEC    6                    3           0       1      1
        //
        // ただ DNS のパケット内で name field は繰り返し登場する一方で、
        // データサイズが大きいのでオリジナルデータへの offset を使って表現する場合がある。
        // この技術は、ドメイン名の圧縮と呼ばれている。

        let mut pos = self.pos();

        // ポインタを使っている場合の処理のための変数
        let mut jumped = false;
        let max_jumps = 5;
        let mut jumps_performed = 0;

        // ドメインの区切り文字 (最初だけ空文字で、それ以降は dot になる)
        let mut delim = "";
        loop {
            // jump を悪用して無限ループが起きるようなパケットでないことを検証する
            if jumps_performed > max_jumps {
                return Err(format!("Limit of {} jumps exceeded", max_jumps).into());
            }

            // label の長さ
            let len = self.get(pos)?;

            // もし len の上位 2 bit が 11 であれば、offset を表している
            if (len & 0xC0) == 0xC0 {
                // offset を表している場合のロジック
                if !jumped {
                    // offset を表している場合は name filed は 2 byte なので、2 byte 分進める
                    self.seek(pos + 2)?;
                }

                // offset の位置を取得する
                let b2 = self.get(pos + 1)? as u16;
                let offset = (((len as u16) ^ 0xC0) << 8) | b2;
                pos = offset as usize;

                // 変数を更新する
                jumped = true;
                jumps_performed += 1;
                continue;
            } else {
                // 基本的な name filed の読み取りロジック
                pos += 1;

                // ドメイン名の長さが 0 の場合は処理を終了する
                if len == 0 {
                    break;
                }

                // 区切り文字を追加して、ドメイン名を読み取る
                outstr.push_str(delim);
                let str_buffer = self.get_range(pos, len as usize)?;
                outstr.push_str(&String::from_utf8_lossy(str_buffer).to_lowercase());

                delim = ".";
                pos += len as usize;
            }
        }

        // jump を利用してない場合は byte を読み取った分だけ進める
        if !jumped {
            self.seek(pos)?;
        }

        Ok(())
    }

    fn write(&mut self, val: u8) -> Result<()> {
        if self.pos >= 512 {
            return Err("End of buffer".into());
        }
        self.buf[self.pos] = val;
        self.pos += 1;
        Ok(())
    }

    pub fn write_u8(&mut self, val: u8) -> Result<()> {
        self.write(val)?;

        Ok(())
    }

    pub fn write_u16(&mut self, val: u16) -> Result<()> {
        self.write((val >> 8) as u8)?;
        self.write((val & 0xFF) as u8)?;

        Ok(())
    }

    pub fn write_u32(&mut self, val: u32) -> Result<()> {
        self.write(((val >> 24) & 0xFF) as u8)?;
        self.write(((val >> 16) & 0xFF) as u8)?;
        self.write(((val >> 8) & 0xFF) as u8)?;
        self.write(((val >> 0) & 0xFF) as u8)?;

        Ok(())
    }

    pub fn write_qname(&mut self, qname: &str) -> Result<()> {
        for label in qname.split('.') {
            let len = label.len();
            if len > 0x3f {
                return Err("Single label exceeds 63 characters of length".into());
            }

            self.write_u8(len as u8)?;
            for b in label.as_bytes() {
                self.write_u8(*b)?;
            }
        }

        self.write_u8(0)?; // 末尾は 0 で埋める

        Ok(())
    }

    fn set(&mut self, pos: usize, val: u8) -> Result<()> {
        self.buf[pos] = val;

        Ok(())
    }

    pub fn set_u16(&mut self, pos: usize, val: u16) -> Result<()> {
        self.set(pos, (val >> 8) as u8)?;
        self.set(pos + 1, (val & 0xFF) as u8)?;

        Ok(())
    }
}
