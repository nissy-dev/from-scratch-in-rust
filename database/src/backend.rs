use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom},
};

use anyhow::{Error, Ok, Result};

use crate::schema::{Column, Schema};

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 100;

pub struct Cursor {
    row_num: usize,
    page_num: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {
            row_num: 0,
            page_num: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    data: [u8; PAGE_SIZE],
    offset: usize,
}

impl Page {
    pub fn new() -> Self {
        Page {
            data: [0; PAGE_SIZE],
            offset: 0,
        }
    }

    pub fn insert(&mut self, data: Vec<u8>) -> Result<(), Error> {
        if self.offset + data.len() > PAGE_SIZE {
            return Err(Error::msg("Page is full"));
        }
        self.data[self.offset..self.offset + data.len()].copy_from_slice(&data);
        self.offset += data.len();
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.offset
    }
}

pub struct Pager {
    file: File,
    pages: [Option<Box<Page>>; MAX_PAGES],
}

const PAGE_DEFAULT_VALUE: Option<Box<Page>> = None;

impl Pager {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        Ok(Pager {
            file: OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(file_path)?,
            pages: [PAGE_DEFAULT_VALUE; MAX_PAGES],
        })
    }

    pub fn get_page(&mut self, page_num: usize) -> Result<&mut Page, Error> {
        if page_num + 1 >= MAX_PAGES {
            return Err(Error::msg("Table is full"));
        }
        // page が None の場合
        if self.pages[page_num].is_none() {
            let mut page = Page::new();
            if self.file.metadata()?.len() >= ((page_num + 1) * PAGE_SIZE) as u64 {
                // 該当の page がファイルに存在する場合は読み込む。
                let mut buffer = [0; PAGE_SIZE];
                let mut reader = BufReader::new(&self.file);
                reader.seek(SeekFrom::Start((page_num * PAGE_SIZE) as u64))?;
                reader.read_exact(&mut buffer)?;
                page.insert(buffer.to_vec())?;
            }
            self.pages[page_num] = Some(Box::new(page));
        }
        self.pages[page_num]
            .as_deref_mut()
            .ok_or_else(|| Error::msg("page not found"))
    }

    pub fn insert_data_topage(&mut self, page_num: usize, data: Vec<u8>) -> Result<(), Error> {
        if let Some(page) = &mut self.pages[page_num] {
            page.insert(data)?;
            Ok(())
        } else {
            Err(Error::msg("page not found"))
        }
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.file.set_len(0)?;
        self.pages = [PAGE_DEFAULT_VALUE; MAX_PAGES];
        Ok(())
    }
}

pub struct Table {
    schema: Schema,
    pager: Pager,
    cursor: Cursor,
}

impl Table {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        Ok(Table {
            schema: Schema::new(),
            pager: Pager::new(file_path)?,
            cursor: Cursor::new(),
        })
    }

    pub fn set_columns(&mut self, columns: Vec<Column>) {
        for col in columns {
            self.schema.add_column(col);
        }
    }

    pub fn set_row(&mut self, row: &[&str]) -> Result<(), Error> {
        if !self.schema.validate_row(row) {
            return Err(Error::msg("Failed to validate row"));
        }
        let serialized_row = self.schema.serialize_row(row)?;
        let max_row_num = PAGE_SIZE / self.schema.row_size();
        if self.cursor.row_num >= max_row_num {
            // page の空きがない場合は次のページへ
            self.cursor.page_num += 1;
            self.cursor.row_num = 0;
        }
        let page = self.pager.get_page(self.cursor.page_num)?;
        page.insert(serialized_row)?;
        self.cursor.row_num += 1;
        Ok(())
    }

    pub fn get_all_rows(&mut self) -> Result<Vec<Vec<String>>, Error> {
        let mut rows = Vec::new();
        let row_size = self.schema.row_size();
        for i in 0..(self.cursor.page_num + 1) {
            let page = self.pager.get_page(i)?;
            let mut offset = 0;
            while offset < page.len() {
                let row_data = &page.data[offset..offset + row_size];
                rows.push(self.schema.deserialize_row(row_data)?);
                offset += row_size;
            }
        }
        Ok(rows)
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.schema = Schema::new();
        self.pager.clear()?;
        self.cursor = Cursor::new();
        Ok(())
    }
}
