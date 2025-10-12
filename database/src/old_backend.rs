use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
};

use anyhow::{Error, Ok, Result};

use crate::schema::{Column, Schema, Value};

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 100;

pub struct Cursor {
    row_num: usize,
    page_num: usize,
}

impl Cursor {
    pub fn new(row_num: usize, page_num: usize) -> Self {
        Cursor { row_num, page_num }
    }

    pub fn as_vec(&self) -> [u8; 2] {
        [self.page_num as u8, self.row_num as u8]
    }

    pub fn from_vec(data: &[u8]) -> Self {
        Cursor {
            page_num: data[0] as usize,
            row_num: data[1] as usize,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OldPage {
    buffer: [u8; PAGE_SIZE],
}

// 最初の 2byte は offset 情報として使用する
impl OldPage {
    pub fn new() -> Self {
        let mut buffer = [0; PAGE_SIZE];
        buffer[0..2].copy_from_slice(&(2u16).to_le_bytes());
        OldPage { buffer }
    }

    pub fn insert(&mut self, data: &[u8]) -> Result<(), Error> {
        let offset = u16::from_le_bytes([self.buffer[0], self.buffer[1]]) as usize;
        if offset + data.len() > PAGE_SIZE {
            return Err(Error::msg("Page is full"));
        }
        self.buffer[offset..offset + data.len()].copy_from_slice(&data);
        self.buffer[0..2].copy_from_slice(&((offset + data.len()) as u16).to_le_bytes());
        Ok(())
    }

    pub fn load(&mut self, buffer: &[u8]) -> Result<(), Error> {
        if buffer.len() != PAGE_SIZE {
            return Err(Error::msg("Invalid page size"));
        }
        self.buffer.copy_from_slice(buffer);
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        &self.buffer[2..]
    }

    pub fn clear(&mut self) {
        self.buffer = Self::new().buffer;
    }
}

pub struct OldPager {
    file: File,
    pages: [Option<Box<OldPage>>; MAX_PAGES],
}

const PAGE_DEFAULT_VALUE: Option<Box<OldPage>> = None;

impl OldPager {
    pub fn new(file: File) -> Self {
        OldPager {
            file,
            pages: [PAGE_DEFAULT_VALUE; MAX_PAGES],
        }
    }

    pub fn get_page(&mut self, page_idx: usize) -> Result<&mut OldPage, Error> {
        if page_idx > (MAX_PAGES - 1) {
            return Err(Error::msg("Table is full"));
        }
        // page が None の場合
        if self.pages[page_idx].is_none() {
            let mut page = OldPage::new();
            if self.file.metadata()?.len() >= ((page_idx + 1) * PAGE_SIZE) as u64 {
                // 該当の page がファイルに存在する場合は読み込む。
                let mut buffer = [0; PAGE_SIZE];
                let mut reader = BufReader::new(&self.file);
                reader.seek(SeekFrom::Start((page_idx * PAGE_SIZE) as u64))?;
                reader.read_exact(&mut buffer)?;
                page.load(&buffer)?;
            }
            self.pages[page_idx] = Some(Box::new(page));
        }
        self.pages[page_idx]
            .as_deref_mut()
            .ok_or_else(|| Error::msg("page not found"))
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        for i in 0..MAX_PAGES {
            if let Some(page) = &self.pages[i] {
                self.file.seek(SeekFrom::Start((i * PAGE_SIZE) as u64))?;
                self.file.write_all(&page.buffer)?;
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.file.set_len(0)?;
        self.pages = [PAGE_DEFAULT_VALUE; MAX_PAGES];
        Ok(())
    }
}

// １ページ目には schema 情報と cursor の情報を保存する
// ２ページ目以降にデータを保存する
pub struct OldTable {
    schema: Schema,
    pager: OldPager,
    cursor: Cursor,
}

impl OldTable {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)?;
        if file.metadata()?.len() == 0 {
            return Ok(OldTable {
                schema: Schema::new(),
                pager: OldPager::new(file),
                cursor: Cursor::new(0, 0),
            });
        }
        let mut pager = OldPager::new(file);
        let page = pager.get_page(0)?;
        let cursor = Cursor::from_vec(&page.data()[0..2]);
        let schema = Schema::deserialize_columns(&page.data()[2..])?;
        Ok(OldTable {
            schema,
            pager,
            cursor,
        })
    }

    pub fn set_schema(&mut self, columns: Vec<Column>) -> Result<(), Error> {
        for col in columns {
            self.schema.add_column(col);
        }
        Ok(())
    }

    pub fn insert(&mut self, tokens: &[&str]) -> Result<(), Error> {
        if !self.schema.is_defined() {
            return Err(Error::msg("Schema is not defined"));
        }
        let values = self.schema.parse_row(tokens)?;
        let row_num_per_page = PAGE_SIZE / self.schema.row_size();
        if self.cursor.row_num != 0 && self.cursor.row_num % row_num_per_page == 0 {
            // page の空きがない場合は次のページへ
            self.cursor.page_num += 1;
        }
        // １ページ目は schema 情報と cursor 情報を保存するため、データは２ページ目から保存する
        let page = self.pager.get_page(self.cursor.page_num + 1)?;
        page.insert(&self.schema.serialize_row_values(&values)?)?;
        self.cursor.row_num += 1;
        Ok(())
    }

    pub fn select_all(&mut self) -> Result<Vec<Vec<Value>>, Error> {
        if !self.schema.is_defined() {
            return Err(Error::msg("Schema is not defined"));
        }
        let mut rows = Vec::new();
        let row_size = self.schema.row_size();
        let row_num_per_page = PAGE_SIZE / row_size;
        for i in 0..=self.cursor.page_num {
            let page = self.pager.get_page(i + 1)?;
            let row_num = if i == self.cursor.page_num {
                self.cursor.row_num % row_num_per_page
            } else {
                row_num_per_page
            };
            for j in 0..row_num {
                let data = &page.data()[j * row_size..(j + 1) * row_size];
                rows.push(self.schema.deserialize_row_values(data)?);
            }
        }
        Ok(rows)
    }

    pub fn save(&mut self) -> Result<(), Error> {
        // schema と cursor の情報を１ページ目に保存する
        let page = self.pager.get_page(0)?;
        page.clear();
        page.insert(&self.cursor.as_vec())?;
        page.insert(&self.schema.serialize_columns())?;
        self.pager.flush()
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.schema = Schema::new();
        self.pager.clear()?;
        self.cursor = Cursor::new(0, 0);
        Ok(())
    }
}
