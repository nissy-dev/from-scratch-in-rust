use anyhow::{Error, Ok, Result};

use crate::backend::Table;

pub struct InputBuffer {
    pub buffer: String,
    pub length: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        InputBuffer {
            buffer: String::new(),
            length: 0,
        }
    }

    pub fn read_input(&mut self) -> Result<(), Error> {
        let mut input = String::new();
        self.length = std::io::stdin().read_line(&mut input)?;
        self.buffer = input.trim().to_string();
        Ok(())
    }

    pub fn is_meta_command(&self) -> bool {
        self.buffer.starts_with('.')
    }
}

pub struct MetaCommand<'a> {
    command: &'a str,
}

impl<'a> MetaCommand<'a> {
    pub fn new(command: &'a str) -> Self {
        MetaCommand { command }
    }

    pub fn execute(&self) -> Result<(), Error> {
        match self.command {
            ".exit" => {
                std::process::exit(0);
            }
            _ => {
                return Err(Error::msg("Unrecognized meta command"));
            }
        }
    }
}

pub struct Statement<'a> {
    content: &'a str,
}

impl<'a> Statement<'a> {
    pub fn new(content: &'a str) -> Self {
        Statement { content }
    }

    pub fn execute(&self, table: &mut Table) -> Result<(), Error> {
        let mut tokens = self.content.split_whitespace().into_iter();
        match tokens.next() {
            // create <column_type> ...
            // ex: create int text(10) text(32)
            Some("create") => {
                let mut columns = Vec::new();
                while let Some(column) = tokens.next() {
                    columns.push(column.try_into()?);
                }
                table.set_columns(columns);
            }
            // insert <value> ...
            // ex) insert 1 hello world
            Some("insert") => {
                let values: Vec<&str> = tokens.collect();
                table.set_row(&values)?;
            }
            // select
            // ex) select
            Some("select") => {
                let rows = table.get_rows()?;
                for (i, row) in rows.iter().enumerate() {
                    println!("row {}: ({})", i, row.join(", "));
                }
            }
            _ => {
                println!("Unrecognized statement: '{}'", self.content);
            }
        }

        Ok(())
    }
}
