use anyhow::{Error, Ok, Result};

use crate::schema::{Column, Schema};

const PAGE_SIZE: usize = 4096;
const TABLE_MAX_PAGES: usize = 100;

pub struct Page {
    data: Vec<u8>,
}

impl Page {
    pub fn new() -> Self {
        Page {
            data: Vec::with_capacity(PAGE_SIZE),
        }
    }

    pub fn insert(&mut self, data: Vec<u8>) -> Result<(), Error> {
        self.data.extend(data);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }
}

pub struct Table {
    schema: Schema,
    pages: Vec<Page>,
}

impl Table {
    pub fn new() -> Self {
        Table {
            schema: Schema::new(),
            pages: Vec::with_capacity(TABLE_MAX_PAGES),
        }
    }

    pub fn set_columns(&mut self, columns: Vec<Column>) {
        for col in columns {
            self.schema.add_column(col);
        }
    }

    pub fn set_row(&mut self, row: &[&str]) -> Result<(), Error> {
        if !self.schema.validate_row(row) {
            return Err(Error::msg("failed to validate row"));
        }
        let serialized_row = self.schema.serialize_row(row)?;
        if self.pages.is_empty() {
            self.pages.push(Page::new());
        }
        let current_page = self.pages.last_mut().unwrap();
        if current_page.len() + serialized_row.len() > current_page.capacity()
            && self.pages.len() <= self.pages.capacity()
        {
            self.pages.push(Page::new());
        }
        self.pages.last_mut().unwrap().insert(serialized_row)?;
        Ok(())
    }

    pub fn get_rows(&self) -> Result<Vec<Vec<String>>, Error> {
        let mut rows = Vec::new();
        let row_size = self.schema.row_size();
        for page in &self.pages {
            let mut offset = 0;
            while offset < page.len() {
                let row_data = &page.data[offset..offset + row_size];
                rows.push(self.schema.deserialize_row(row_data)?);
                offset += row_size;
            }
        }
        Ok(rows)
    }

    pub fn reset(&mut self) {
        self.schema = Schema::new();
        self.pages.clear();
        println!("table reset.");
    }
}
