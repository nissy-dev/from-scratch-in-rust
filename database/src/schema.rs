use anyhow::{Error, Ok, Result};

// 'int' or 'text(size)'
#[derive(Debug, Clone)]
pub enum Column {
    Integer, // 32bit
    Text(usize),
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

impl Column {
    pub fn size(&self) -> usize {
        match self {
            Column::Integer => 4,
            Column::Text(size) => *size,
        }
    }

    pub fn validate(&self, value: &str) -> bool {
        match self {
            Column::Integer => value.parse::<i32>().is_ok(),
            Column::Text(size) => value.len() <= *size,
        }
    }

    pub fn serialize(&self, value: &str) -> Result<Vec<u8>, Error> {
        match self {
            Column::Integer => {
                let int_value = value
                    .parse::<i32>()
                    .map_err(|_| Error::msg("Invalid integer value"))?;
                Ok(int_value.to_le_bytes().to_vec())
            }
            Column::Text(size) => {
                let mut bytes = vec![0; *size];
                let value_bytes = value.as_bytes();
                bytes[..value_bytes.len()].copy_from_slice(&value_bytes);
                Ok(bytes)
            }
        }
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<String, Error> {
        match self {
            Column::Integer => {
                if data.len() != 4 {
                    return Err(Error::msg("Invalid data length for integer"));
                }
                let int_bytes = [data[0], data[1], data[2], data[3]];
                let int_value = i32::from_le_bytes(int_bytes);
                Ok(int_value.to_string())
            }
            Column::Text(size) => {
                if data.len() != *size {
                    return Err(Error::msg("Invalid data length for text"));
                }
                let text_value = String::from_utf8(data.to_vec())
                    .map_err(|_| Error::msg("Invalid UTF-8 sequence"))?;
                // 0 を取り除く処理が必要
                Ok(text_value.trim_end_matches(char::from(0)).to_string())
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

    pub fn add_column(&mut self, column: Column) {
        self.columns.push(column);
    }

    pub fn validate_row(&self, row: &[&str]) -> bool {
        if self.columns.len() != row.len() {
            return false;
        }

        for (col, value) in self.columns.iter().zip(row.iter()) {
            if !col.validate(value) {
                return false;
            }
        }
        true
    }

    pub fn serialize_row(&self, row: &[&str]) -> Result<Vec<u8>, Error> {
        let mut serialized = Vec::new();
        for (col, value) in self.columns.iter().zip(row.iter()) {
            serialized.extend(col.serialize(value)?);
        }
        Ok(serialized)
    }

    pub fn deserialize_row(&self, data: &[u8]) -> Result<Vec<String>, Error> {
        let mut offset = 0;
        let mut row = Vec::new();
        for col in &self.columns {
            let col_size = col.size();
            row.push(col.deserialize(&data[offset..offset + col_size])?);
            offset += col_size;
        }
        Ok(row)
    }

    pub fn row_size(&self) -> usize {
        self.columns.iter().map(|col| col.size()).sum()
    }
}
