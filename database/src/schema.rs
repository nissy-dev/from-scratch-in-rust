use anyhow::{Error, Ok, Result};
use core::str;
use std::fmt;

const COLUMN_INTEGER: u8 = 1;
const COLUMN_TEXT: u8 = 2;

// 'int' or 'text(size)'
#[derive(Debug, Clone)]
pub enum Column {
    Integer,  // 32bit
    Text(u8), // size は最大で 255 (固定長文字列格納)
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i32),
    Text(String),
}

impl TryFrom<&str> for Column {
    type Error = Error;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "int" => Ok(Column::Integer),
            _ if name.starts_with("text") => {
                let size = name
                    .trim_start_matches("text(")
                    .trim_end_matches(")")
                    .parse()?;
                Ok(Column::Text(size))
            }
            _ => Err(Error::msg("Unknown column type")),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Text(s) => write!(f, "{}", s),
        }
    }
}

impl Column {
    pub fn size(&self) -> usize {
        match self {
            Column::Integer => 4,
            Column::Text(size) => *size as usize,
        }
    }

    pub fn parse_value(&self, raw_str_value: &str) -> Result<Value, Error> {
        match self {
            Column::Integer => {
                let v = raw_str_value
                    .parse::<i32>()
                    .map_err(|_| Error::msg("Invalid integer value"))?;
                Ok(Value::Integer(v))
            }
            Column::Text(size) => {
                if raw_str_value.len() > *size as usize {
                    return Err(Error::msg("Text too long"));
                }
                Ok(Value::Text(raw_str_value.to_string()))
            }
        }
    }

    pub fn serialize_value(&self, value: &Value) -> Result<Vec<u8>, Error> {
        match (self, value) {
            (Column::Integer, Value::Integer(i)) => Ok(i.to_le_bytes().to_vec()),
            (Column::Text(size), Value::Text(s)) => {
                let mut buf = vec![0u8; *size as usize];
                let str_bytes = s.as_bytes();
                buf[..str_bytes.len()].copy_from_slice(str_bytes);
                Ok(buf)
            }
            _ => Err(Error::msg("Column / Value type mismatch")),
        }
    }

    pub fn deserialize_value(&self, data: &[u8]) -> Result<Value, Error> {
        match self {
            Column::Integer => {
                if data.len() != 4 {
                    return Err(Error::msg("Invalid data length for integer"));
                }
                let mut arr = [0u8; 4];
                arr.copy_from_slice(&data[0..4]);
                Ok(Value::Integer(i32::from_le_bytes(arr)))
            }
            Column::Text(size) => {
                if data.len() != *size as usize {
                    return Err(Error::msg("Invalid data length for text"));
                }
                let s = String::from_utf8(data.to_vec())
                    .map_err(|_| Error::msg("Invalid UTF-8 sequence"))?;
                Ok(Value::Text(s.trim_end_matches(char::from(0)).to_string()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    columns: Vec<Column>,
}

impl Schema {
    pub fn new() -> Self {
        Schema {
            columns: Vec::new(),
        }
    }

    pub fn is_defined(&self) -> bool {
        !self.columns.is_empty()
    }

    pub fn add_column(&mut self, column: Column) {
        self.columns.push(column);
    }

    pub fn parse_row(&self, tokens: &[&str]) -> Result<Vec<Value>, Error> {
        if self.columns.len() != tokens.len() {
            return Err(Error::msg("Column count mismatch"));
        }
        self.columns
            .iter()
            .zip(tokens.iter())
            .map(|(c, t)| c.parse_value(t))
            .collect()
    }

    // スキーマの行定義をバイナリ形式でシリアライズする
    // int -> COLUMN_INTEGER
    // text(size) -> [COLUMN_TEXT, size]
    pub fn serialize_columns(&self) -> Vec<u8> {
        let mut serialized = Vec::new();
        for col in &self.columns {
            match col {
                Column::Integer => serialized.push(COLUMN_INTEGER),
                Column::Text(size) => serialized.extend_from_slice(&[COLUMN_TEXT, *size as u8]),
            }
        }
        serialized
    }

    pub fn deserialize_columns(data: &[u8]) -> Result<Self, Error> {
        let mut schema = Self::new();
        let mut idx = 0;
        while idx < data.len() {
            match data[idx] {
                COLUMN_INTEGER => {
                    schema.add_column(Column::Integer);
                    idx += 1;
                }
                COLUMN_TEXT => {
                    let size = data[idx + 1];
                    schema.add_column(Column::Text(size));
                    idx += 2;
                }
                _ => {
                    break;
                }
            }
        }
        Ok(schema)
    }

    pub fn serialize_row_values(&self, values: &[Value]) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        for (col, value) in self.columns.iter().zip(values.iter()) {
            out.extend(col.serialize_value(value)?);
        }
        Ok(out)
    }

    pub fn deserialize_row_values(&self, data: &[u8]) -> Result<Vec<Value>, Error> {
        let mut offset = 0;
        let mut row = Vec::new();
        for col in &self.columns {
            let sz = col.size();
            row.push(col.deserialize_value(&data[offset..offset + sz])?);
            offset += sz;
        }
        Ok(row)
    }

    pub fn row_size(&self) -> usize {
        self.columns.iter().map(|col| col.size()).sum()
    }
}
