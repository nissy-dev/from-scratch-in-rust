use crate::value::Value;

const TABLE_MAX_LOAD: f64 = 0.75;

#[derive(Debug, Clone)]
struct Entry {
    key: Option<String>,
    value: Value,
    is_deleted: bool,
}

impl Entry {
    fn new() -> Self {
        Entry {
            key: None,
            value: Value::Nil,
            is_deleted: false,
        }
    }

    fn is_initial(&self) -> bool {
        self.key.is_none() && self.value == Value::Nil && !self.is_deleted
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    count: usize,
    capacity: usize,
    entries: Vec<Entry>,
}

impl Table {
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        entries.resize_with(capacity, || Entry::new());

        Table {
            count: 0,
            capacity,
            entries,
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        if (self.count + 1) as f64 > self.capacity as f64 * TABLE_MAX_LOAD {
            self.resize(self.capacity * 2);
        }

        let index = self.find_entry(&key);
        let entry = &mut self.entries[index];
        if entry.is_initial() {
            self.count += 1;
        }

        entry.key = Some(key.to_string());
        entry.value = value;
        // 論理削除されたエントリを再利用するために、フラグをリセットする
        entry.is_deleted = false;
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        let index = self.find_entry(&key);
        let entry = &self.entries[index];
        if entry.key.is_some() && !entry.is_deleted {
            Some(&entry.value)
        } else {
            None
        }
    }

    pub fn delete(&mut self, key: &str) -> bool {
        let index = self.find_entry(&key);
        let entry = &mut self.entries[index];
        if entry.key.is_none() {
            return false;
        }

        // hash が衝突した場合は、次のエントリに入れるようにしているので、すぐにエントリは消せない
        // 代わりに論理削除する
        entry.key = None;
        entry.is_deleted = true;
        true
    }

    fn hash(&self, key: &str) -> usize {
        let mut hash = 2166136261u32;
        for byte in key.as_bytes() {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash as usize
    }

    fn find_entry(&self, key: &str) -> usize {
        let mut index = self.hash(&key) % self.capacity;
        let mut tombstone_index = None;
        loop {
            let entry = &self.entries[index];
            if entry.is_deleted {
                // 論理削除されているエントリの場合は、その場所を記録しておく
                tombstone_index.get_or_insert(index);
            } else {
                if let Some(stored_key) = &entry.key {
                    if stored_key == key {
                        return index;
                    }
                } else {
                    return tombstone_index.unwrap_or(index);
                }
            }
            index = (index + 1) % self.capacity;
        }
    }

    fn resize(&mut self, new_capacity: usize) {
        let mut new_table = Table::new(new_capacity);

        for entry in &self.entries {
            if let Some(key) = &entry.key {
                if !entry.is_deleted {
                    new_table.set(&key, entry.value.clone());
                }
            }
        }

        *self = new_table;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::*;

    #[test]
    fn test() {
        let mut table = Table::new(8);

        let key = "name";
        let string = Object::String("Rust".to_string());
        let value = Value::Object(string);
        table.set(key, value.clone());

        let entry = table.get(&key);
        assert_eq!(entry.is_some(), true);
        assert_eq!(entry.unwrap(), &value);

        // update
        let new_string = Object::String("Lox".to_string());
        let new_value = Value::Object(new_string);
        table.set(&key, new_value.clone());

        let entry = table.get(&key);
        assert_eq!(entry.is_some(), true);
        assert_eq!(entry.unwrap(), &new_value);

        // delete
        let deleted = table.delete(&key);
        assert_eq!(deleted, true);
        let entry = table.get(&key);
        assert_eq!(entry.is_none(), true);
    }

    #[test]
    fn test_capacity() {
        let mut table = Table::new(4);
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = Value::Number(i as f64);
            table.set(&key, value);
        }
        assert!(table.capacity >= 10, "Table should have resized");
        assert!(table.count == 10, "Table should have ten elements");
    }
}
