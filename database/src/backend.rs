use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Error, Ok, Result};

use crate::schema::{Column, Schema, Value};

const PAGE_SIZE: usize = 4096;
const MAX_PAGES: usize = 100;

const PARENT_POINTER_NONE: u32 = u32::MAX;

#[derive(Debug)]
enum BTreeNode {
    Internal(BTreeInternal),
    Leaf(BTreeLeaf),
}

#[derive(Debug)]
struct BTreeInternal {
    is_root: bool,
    parent: Option<u32>,
    keys: Vec<u32>,
    children: Vec<u32>, // children[i] < keys[i] < children[i+1]
}

impl BTreeInternal {
    fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buffer = [0u8; PAGE_SIZE];
        buffer[0] = 0;
        buffer[1] = self.is_root as u8;
        let parent_pointer = self.parent.unwrap_or(PARENT_POINTER_NONE);
        buffer[2..6].copy_from_slice(&parent_pointer.to_le_bytes());
        buffer[6..10].copy_from_slice(&(self.keys.len() as u32).to_le_bytes());
        let mut offset = 10;
        for key in self.keys.iter() {
            buffer[offset..offset + 4].copy_from_slice(&key.to_le_bytes());
            offset += 4;
        }
        buffer[offset..offset + 4].copy_from_slice(&(self.children.len() as u32).to_le_bytes());
        offset += 4;
        for child in self.children.iter() {
            buffer[offset..offset + 4].copy_from_slice(&child.to_le_bytes());
            offset += 4;
        }
        buffer
    }

    fn deserialize(buffer: &[u8; PAGE_SIZE]) -> Self {
        let is_root = buffer[1] != 0;
        let parent = u32::from_le_bytes(buffer[2..6].try_into().unwrap());
        let parent = if parent == PARENT_POINTER_NONE {
            None
        } else {
            Some(parent)
        };
        let mut keys = Vec::new();
        let mut children = Vec::new();
        let mut offset = 10;
        let num_keys = u32::from_le_bytes(buffer[6..10].try_into().unwrap());
        for _ in 0..num_keys {
            keys.push(u32::from_le_bytes(
                buffer[offset..offset + 4].try_into().unwrap(),
            ));
            offset += 4;
        }
        let num_children = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
        for _ in 0..num_children {
            children.push(u32::from_le_bytes(
                buffer[offset..offset + 4].try_into().unwrap(),
            ));
            offset += 4;
        }
        Self {
            is_root,
            parent,
            keys,
            children,
        }
    }
}

#[derive(Debug)]
struct BTreeLeaf {
    is_root: bool,
    parent: Option<u32>,
    cells: BTreeMap<u32, Vec<u8>>,
}

impl BTreeLeaf {
    fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buffer = [0u8; PAGE_SIZE];
        buffer[0] = 1;
        buffer[1] = self.is_root as u8;
        let parent_pointer = self.parent.unwrap_or(PARENT_POINTER_NONE);
        buffer[2..6].copy_from_slice(&parent_pointer.to_le_bytes());
        buffer[6..10].copy_from_slice(&(self.cells.len() as u32).to_le_bytes());
        let mut offset = 10;
        for (key, value) in self.cells.iter() {
            buffer[offset..offset + 4].copy_from_slice(&key.to_le_bytes());
            offset += 4;
            buffer[offset..offset + 4].copy_from_slice(&(value.len() as u32).to_le_bytes());
            offset += 4;
            buffer[offset..offset + value.len()].copy_from_slice(value);
            offset += value.len();
        }
        buffer
    }

    fn deserialize(buffer: &[u8; PAGE_SIZE]) -> Self {
        let is_root = buffer[1] != 0;
        let parent = u32::from_le_bytes(buffer[2..6].try_into().unwrap());
        let parent = if parent == PARENT_POINTER_NONE {
            None
        } else {
            Some(parent)
        };
        let num_cells = u32::from_le_bytes(buffer[6..10].try_into().unwrap());
        let mut cells = BTreeMap::new();
        let mut offset = 10;
        for _ in 0..num_cells {
            let key = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
            offset += 4;
            let value_len = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
            offset += 4;
            let value = buffer[offset..offset + value_len as usize].to_vec();
            offset += value_len as usize;
            cells.insert(key, value);
        }
        Self {
            is_root,
            parent,
            cells,
        }
    }

    fn can_hold(&self, value: &[u8]) -> bool {
        let used_space: usize = 10
            + self
                .cells
                .values()
                .map(|v| 4 + 4 + v.len()) // key(4) + value_len(4) + value
                .sum::<usize>();
        used_space + 4 + 4 + value.len() <= PAGE_SIZE
    }

    fn insert(&mut self, key: u32, value: Vec<u8>) -> Result<(), Error> {
        if self.cells.contains_key(&key) {
            return Err(Error::msg("Duplicate key"));
        }
        if !self.can_hold(&value) {
            return Err(Error::msg("Leaf node is full"));
        }
        self.cells.insert(key, value);
        Ok(())
    }
}

impl BTreeNode {
    fn new_root() -> Self {
        BTreeNode::Leaf(BTreeLeaf {
            is_root: true,
            parent: None,
            cells: BTreeMap::new(),
        })
    }

    fn serialize(&self) -> [u8; PAGE_SIZE] {
        match self {
            BTreeNode::Internal(internal) => internal.serialize(),
            BTreeNode::Leaf(leaf) => leaf.serialize(),
        }
    }

    fn deserialize(buffer: &[u8; PAGE_SIZE]) -> Self {
        match buffer[0] {
            0 => BTreeNode::Internal(BTreeInternal::deserialize(buffer)),
            1 => BTreeNode::Leaf(BTreeLeaf::deserialize(buffer)),
            _ => panic!("Invalid node type"),
        }
    }
}

struct Pager {
    file: File,
    cache: HashMap<u32, BTreeNode>,
    num_pages: u32,
}

impl Pager {
    pub const META_PAGE_SIZE: usize = PAGE_SIZE;

    pub fn new(file: File, num_pages: u32) -> Result<Self, Error> {
        Ok(Self {
            file,
            cache: HashMap::new(),
            num_pages,
        })
    }

    pub fn allocate_node(&mut self, node: BTreeNode) -> Result<u32> {
        if self.num_pages as usize >= MAX_PAGES {
            return Err(Error::msg("Maximum number of pages reached"));
        }
        self.num_pages += 1;
        let page_id = self.num_pages - 1;
        self.cache.insert(page_id, node);
        Ok(page_id)
    }

    pub fn get_page(&mut self, page_id: u32) -> Result<&mut BTreeNode> {
        match self.cache.entry(page_id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let mut buf = [0u8; PAGE_SIZE];
                // meta ページを考慮して seek する
                self.file
                    .seek(SeekFrom::Start((page_id + 1) as u64 * PAGE_SIZE as u64))?;
                self.file.read_exact(&mut buf)?;
                let node = BTreeNode::deserialize(&buf);
                Ok(entry.insert(node))
            }
        }
    }

    pub fn flush(&mut self, meta_page: &[u8; PAGE_SIZE]) -> Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(meta_page)?;
        for (page_id, node) in self.cache.iter() {
            let buf = node.serialize();
            // meta ページを考慮して seek する
            self.file
                .seek(SeekFrom::Start((page_id + 1) as u64 * PAGE_SIZE as u64))?;
            self.file.write_all(&buf)?;
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.file.set_len(0)?;
        self.cache.clear();
        self.num_pages = 0;
        Ok(())
    }
}

pub struct Table {
    schema: Schema,
    pager: Pager,
}

impl Table {
    const BTREE_ROOT_PAGE_ID: u32 = 0;

    pub fn new(path: &str) -> Result<Self, Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let len = file.metadata()?.len();
        let num_pages = (len / PAGE_SIZE as u64) as u32;
        if num_pages >= 1 {
            // 1ページ目にスキーマ情報が保存されているので読み込む
            let mut buf = [0u8; PAGE_SIZE];
            file.seek(SeekFrom::Start(0))?;
            file.read_exact(&mut buf)?;
            let schema = Schema::deserialize_columns(&buf)?;
            let mut pager = Pager::new(file, num_pages - 1)?;
            // もしBツリーのルートページが存在しなければ初期化する
            if num_pages == 1 {
                pager.allocate_node(BTreeNode::new_root())?;
            }
            Ok(Self { schema, pager })
        } else {
            let mut pager = Pager::new(file, 0)?;
            pager.allocate_node(BTreeNode::new_root())?;
            Ok(Self {
                schema: Schema::new(),
                pager: pager,
            })
        }
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
        let key = match &values[0] {
            Value::Integer(i) => *i,
            // 制約：実装を簡単にするためにカラムの最初を主キーかつint型に限定する
            _ => return Err(Error::msg("First column must be int (primary key)")),
        } as u32;
        let row_bytes = self.schema.serialize_row_values(&values)?;

        let node = self.pager.get_page(Self::BTREE_ROOT_PAGE_ID)?;
        // TODO: 本当はツーリーを捜査して、適切なリーフノードを見つける必要がある
        let leaf_node = match node {
            BTreeNode::Leaf(leaf) => leaf,
            _ => todo!("Implement insertion for non-leaf nodes"),
        };

        // 新しいセルを追加
        // TODO: ノードがいっぱいなら分割する必要がある
        leaf_node.insert(key, row_bytes)?;

        Ok(())
    }

    pub fn select_all(&mut self) -> Result<Vec<Vec<Value>>, Error> {
        let mut results = Vec::new();
        let node = self.pager.get_page(Self::BTREE_ROOT_PAGE_ID)?;
        // TODO: 本当はツリー全体を捜査して、すべてのリーフノードの値を取得する必要がある
        let leaf_node = match node {
            BTreeNode::Leaf(leaf) => leaf,
            _ => todo!("Implement selection for non-leaf nodes"),
        };
        for value_bytes in leaf_node.cells.values() {
            results.push(self.schema.deserialize_row_values(value_bytes)?);
        }
        Ok(results)
    }

    pub fn save(&mut self) -> Result<(), Error> {
        let mut meta_page = [0u8; PAGE_SIZE];
        let schema_bytes = self.schema.serialize_columns();
        meta_page[..schema_bytes.len()].copy_from_slice(&schema_bytes);
        self.pager.flush(&meta_page)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.schema.clear();
        self.pager.clear()?;
        Ok(())
    }
}
