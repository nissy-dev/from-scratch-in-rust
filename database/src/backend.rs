use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    vec,
};

use anyhow::{Error, Ok, Result};

use crate::schema::{Column, Schema};

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 100;

pub struct Cursor {
    row_idx: usize,
    page_idx: usize,
}

impl Cursor {
    pub fn new(row_idx: usize, page_idx: usize) -> Self {
        Cursor { row_idx, page_idx }
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

    pub fn insert_by_offset(&mut self, data: Vec<u8>, offset: usize) -> Result<(), Error> {
        if offset + data.len() > PAGE_SIZE {
            return Err(Error::msg("Page is full"));
        }
        self.data[offset..offset + data.len()].copy_from_slice(&data);
        Ok(())
    }
}

// ページを管理する構造体
// １ページ目には schema 情報と cursor の情報を保存する
// ２ページ目以降にデータを保存する
pub struct Pager {
    file: File,
    pages: [Option<Box<Page>>; MAX_PAGES],
}

const PAGE_DEFAULT_VALUE: Option<Box<Page>> = None;

impl Pager {
    pub fn new(file: File) -> Self {
        Pager {
            file,
            pages: [PAGE_DEFAULT_VALUE; MAX_PAGES],
        }
    }

    pub fn get_page(&mut self, page_idx: usize) -> Result<&mut Page, Error> {
        if page_idx + 1 >= MAX_PAGES {
            return Err(Error::msg("Table is full"));
        }
        // page が None の場合
        if self.pages[page_idx].is_none() {
            let mut page = Page::new();
            if self.file.metadata()?.len() >= ((page_idx + 1) * PAGE_SIZE) as u64 {
                // 該当の page がファイルに存在する場合は読み込む。
                let mut buffer = [0; PAGE_SIZE];
                let mut reader = BufReader::new(&self.file);
                reader.seek(SeekFrom::Start((page_idx * PAGE_SIZE) as u64))?;
                reader.read_exact(&mut buffer)?;
                page.insert(buffer.to_vec())?;
            }
            self.pages[page_idx] = Some(Box::new(page));
        }
        self.pages[page_idx]
            .as_deref_mut()
            .ok_or_else(|| Error::msg("page not found"))
    }

    pub fn flush(&mut self, page_idx: usize, row_idx: usize) -> Result<(), Error> {
        let page = self.get_page(0)?;
        page.insert_by_offset(vec![page_idx as u8, row_idx as u8], 0)?;
        for i in 0..=page_idx {
            if let Some(page) = &self.pages[i] {
                self.file.seek(SeekFrom::Start((i * PAGE_SIZE) as u64))?;
                self.file.write_all(&page.data)?;
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

pub struct Table {
    schema: Schema,
    pager: Pager,
    cursor: Cursor,
}

impl Table {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_path)?;
        if file.metadata()?.len() == 0 {
            Ok(Table {
                schema: Schema::new(),
                pager: Pager::new(file),
                cursor: Cursor::new(0, 0),
            })
        } else {
            let mut schema = Schema::new();
            let mut pager = Pager::new(file);
            let page = pager.get_page(0)?;
            let page_idx = page.data[0] as usize;
            let row_idx = page.data[1] as usize;
            schema.deserialize_columns(&page.data[2..])?;
            Ok(Table {
                schema,
                pager,
                cursor: Cursor::new(row_idx, page_idx),
            })
        }
    }

    pub fn set_columns(&mut self, columns: Vec<Column>) -> Result<(), Error> {
        for col in columns {
            self.schema.add_column(col);
        }
        let page = self.pager.get_page(0)?;
        page.insert_by_offset(self.schema.serialize_columns(), 2)?;
        self.cursor.page_idx += 1;
        Ok(())
    }

    pub fn set_row(&mut self, row: &[&str]) -> Result<(), Error> {
        if !self.schema.is_defined() {
            return Err(Error::msg("Schema is not defined"));
        }
        if !self.schema.validate_row(row) {
            return Err(Error::msg("Failed to validate row"));
        }
        let serialized_row = self.schema.serialize_row(row)?;
        let max_row_num = PAGE_SIZE / self.schema.row_size();
        if self.cursor.row_idx >= max_row_num {
            // page の空きがない場合は次のページへ
            self.cursor.page_idx += 1;
            self.cursor.row_idx = 0;
        }
        let page = self.pager.get_page(self.cursor.page_idx)?;
        page.insert(serialized_row)?;
        self.cursor.row_idx += 1;
        Ok(())
    }

    pub fn get_all_rows(&mut self) -> Result<Vec<Vec<String>>, Error> {
        if !self.schema.is_defined() {
            return Err(Error::msg("Schema is not defined"));
        }
        let mut rows = Vec::new();
        let row_size = self.schema.row_size();
        let max_row_num = PAGE_SIZE / row_size;
        for i in 1..=self.cursor.page_idx {
            let page = self.pager.get_page(i)?;
            if i == self.cursor.page_idx {
                // 最終ページは row_idx まで読む
                for j in 0..self.cursor.row_idx {
                    let data = &page.data[j * row_size..(j + 1) * row_size];
                    rows.push(self.schema.deserialize_row(data)?);
                }
            } else {
                // それ以外は max_row_num だけ読む
                for j in 0..max_row_num {
                    let data = &page.data[j * row_size..(j + 1) * row_size];
                    rows.push(self.schema.deserialize_row(data)?);
                }
            }
        }
        Ok(rows)
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.pager.flush(self.cursor.page_idx, self.cursor.row_idx)
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.schema = Schema::new();
        self.pager.clear()?;
        self.cursor = Cursor::new(0, 0);
        Ok(())
    }
}
