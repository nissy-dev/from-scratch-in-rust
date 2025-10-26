use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    result::Result::Ok,
};

use anyhow::{Error, Result};

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
    parent: Option<u32>,
    keys: Vec<u32>,
    children: Vec<u32>, // children[i] < keys[i] < children[i+1]
}

impl BTreeInternal {
    fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buffer = [0u8; PAGE_SIZE];
        buffer[0] = 0;
        let parent_pointer = self.parent.unwrap_or(PARENT_POINTER_NONE);
        buffer[1..5].copy_from_slice(&parent_pointer.to_le_bytes());
        buffer[5..9].copy_from_slice(&(self.keys.len() as u32).to_le_bytes());
        let mut offset = 9;
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
        let parent = u32::from_le_bytes(buffer[1..5].try_into().unwrap());
        let parent = if parent == PARENT_POINTER_NONE {
            None
        } else {
            Some(parent)
        };
        let mut keys = Vec::new();
        let mut children = Vec::new();
        let num_keys = u32::from_le_bytes(buffer[5..9].try_into().unwrap());
        let mut offset = 9;
        for _ in 0..num_keys {
            keys.push(u32::from_le_bytes(
                buffer[offset..offset + 4].try_into().unwrap(),
            ));
            offset += 4;
        }
        let num_children = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
        offset += 4;
        for _ in 0..num_children {
            children.push(u32::from_le_bytes(
                buffer[offset..offset + 4].try_into().unwrap(),
            ));
            offset += 4;
        }
        Self {
            parent,
            keys,
            children,
        }
    }
}

#[derive(Debug)]
struct BTreeLeaf {
    parent: Option<u32>,
    keys: Vec<u32>,
    values: Vec<Vec<u8>>,
    next_leaf: Option<u32>,
}

impl BTreeLeaf {
    fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buffer = [0u8; PAGE_SIZE];
        buffer[0] = 1;
        let parent_pointer = self.parent.unwrap_or(PARENT_POINTER_NONE);
        buffer[1..5].copy_from_slice(&parent_pointer.to_le_bytes());
        let next_leaf_pointer = self.next_leaf.unwrap_or(PARENT_POINTER_NONE);
        buffer[5..9].copy_from_slice(&next_leaf_pointer.to_le_bytes());
        buffer[9..13].copy_from_slice(&(self.keys.len() as u32).to_le_bytes());
        let mut offset = 13;
        for (key, value) in self.keys.iter().zip(self.values.iter()) {
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
        let parent = u32::from_le_bytes(buffer[1..5].try_into().unwrap());
        let parent = if parent == PARENT_POINTER_NONE {
            None
        } else {
            Some(parent)
        };
        let next_leaf = u32::from_le_bytes(buffer[5..9].try_into().unwrap());
        let next_leaf = if next_leaf == PARENT_POINTER_NONE {
            None
        } else {
            Some(next_leaf)
        };
        let num_cells = u32::from_le_bytes(buffer[9..13].try_into().unwrap());
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut offset = 13;
        for _ in 0..num_cells {
            let key = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
            offset += 4;
            keys.push(key);
            let value_len = u32::from_le_bytes(buffer[offset..offset + 4].try_into().unwrap());
            offset += 4;
            let value = buffer[offset..offset + value_len as usize].to_vec();
            offset += value_len as usize;
            values.push(value);
        }
        Self {
            parent,
            keys,
            values,
            next_leaf,
        }
    }

    fn is_full_node(&self, value: &[u8]) -> bool {
        let used_space: usize = 13
            + self
                .values
                .iter()
                .map(|v| 4 + 4 + v.len()) // key(4) + value_len(4) + value
                .sum::<usize>();
        used_space + 4 + 4 + value.len() > PAGE_SIZE
    }

    fn insert(&mut self, key: u32, value: Vec<u8>) -> Result<()> {
        if self.keys.contains(&key) {
            return Err(Error::msg("Duplicate key"));
        }
        let pos = self.keys.binary_search(&key).unwrap_or_else(|e| e);
        self.keys.insert(pos, key);
        self.values.insert(pos, value);
        Ok(())
    }

    fn split(&mut self) -> (u32, BTreeLeaf) {
        let mid_index = self.keys.len() / 2;
        // 配列の後半を新しいノードにする
        let new_keys = self.keys.split_off(mid_index);
        // ノードの分割キーは新しいノードの最初のキーとなる
        let split_key = new_keys[0];
        let new_values = self.values.split_off(mid_index);
        let new_right_leaf = BTreeLeaf {
            parent: self.parent,
            keys: new_keys,
            values: new_values,
            next_leaf: None,
        };
        (split_key, new_right_leaf)
    }
}

impl BTreeNode {
    fn new_leaf_root() -> Self {
        BTreeNode::Leaf(BTreeLeaf {
            parent: None,
            keys: Vec::new(),
            values: Vec::new(),
            next_leaf: None,
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
    pager: RefCell<Pager>,
    root_page_id: u32,
}

impl Table {
    pub fn new(path: &str) -> Result<Self, Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let len = file.metadata()?.len();
        let num_pages = (len / PAGE_SIZE as u64) as u32;
        if num_pages == 0 {
            // 新規ファイル: メタページ未作成
            let mut pager = Pager::new(file, 0)?;
            let root_page_id = pager.allocate_node(BTreeNode::new_leaf_root())?;
            return Ok(Self {
                schema: Schema::new(),
                pager: RefCell::new(pager),
                root_page_id,
            });
        }
        // 1ページ目にルートページIDとスキーマ情報が保存されているので読み込む
        let mut buf = [0u8; PAGE_SIZE];
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut buf)?;
        let (mut root_page_id, schema) = Self::deserialize_meta_page(&buf)?;
        let mut pager = Pager::new(file, num_pages - 1)?;
        // もしBツリーのルートページが存在しなければ初期化する
        if num_pages == 1 {
            root_page_id = pager.allocate_node(BTreeNode::new_leaf_root())?;
        }
        Ok(Self {
            schema,
            pager: RefCell::new(pager),
            root_page_id,
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
        let key = match &values[0] {
            Value::Integer(i) => *i,
            // 制約：実装を簡単にするためにカラムの最初を主キーかつint型に限定する
            _ => return Err(Error::msg("First column must be int (primary key)")),
        } as u32;
        let row_bytes = self.schema.serialize_row_values(&values)?;
        let leaf_page_id = self.find_leaf_node_page_id(self.root_page_id, key)?;

        let (split_key, right_leaf) = {
            let mut pager = self.pager.borrow_mut();
            let leaf_node = match pager.get_page(leaf_page_id)? {
                BTreeNode::Leaf(leaf) => leaf,
                _ => return Err(Error::msg("Expected leaf node")),
            };

            // 空きがあれば挿入して早期で処理終了
            if !leaf_node.is_full_node(&row_bytes) {
                leaf_node.insert(key, row_bytes)?;
                return Ok(());
            }

            // 空きがなければノード分割を行う
            leaf_node.insert(key, row_bytes)?;
            let (split_key, right_leaf) = leaf_node.split();
            (split_key, right_leaf)
        };

        // ノード分割後の処理
        if leaf_page_id == self.root_page_id {
            // leaf ノードがルートノードの場合、新しいルートノード (BTreeInternal) を作成する
            self.create_new_leaf_and_internal(leaf_page_id, split_key, right_leaf)?;
        } else {
            // 通常の leaf ノード分割処理
            self.create_new_leaf(leaf_page_id, split_key, right_leaf)?;
        }

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.pager.borrow_mut().flush(&self.serialize_meta_page())?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.schema.clear();
        self.pager.borrow_mut().clear()?;
        Ok(())
    }

    fn find_leaf_node_page_id(&mut self, root_page_id: u32, key: u32) -> Result<u32, Error> {
        let mut page_id = root_page_id;
        loop {
            let mut pager = self.pager.borrow_mut();
            match pager.get_page(page_id)? {
                BTreeNode::Leaf(_) => return Ok(page_id),
                BTreeNode::Internal(internal) => {
                    let pos = match internal.keys.binary_search(&key) {
                        Ok(i) => i + 1,
                        Err(i) => i,
                    };
                    page_id = internal.children[pos];
                }
            }
        }
    }

    fn create_new_leaf_and_internal(
        &mut self,
        left_page_id: u32,
        split_key: u32,
        right_leaf: BTreeLeaf,
    ) -> Result<(), Error> {
        let mut pager = self.pager.borrow_mut();
        // 右の新しいリーフノードを割り当てる
        let right_page_id = pager.allocate_node(BTreeNode::Leaf(right_leaf))?;
        // 新しいルートノードを作成する
        let new_root = BTreeInternal {
            parent: None,
            keys: vec![split_key],
            children: vec![left_page_id, right_page_id],
        };
        let new_root_page_id = pager.allocate_node(BTreeNode::Internal(new_root))?;
        // 左右のリーフノードを更新する
        if let BTreeNode::Leaf(left_leaf) = pager.get_page(left_page_id)? {
            left_leaf.parent = Some(new_root_page_id);
            left_leaf.next_leaf = Some(right_page_id);
        }
        if let BTreeNode::Leaf(right_leaf) = pager.get_page(right_page_id)? {
            right_leaf.parent = Some(new_root_page_id);
        }
        // ルートページIDを更新する
        self.root_page_id = new_root_page_id;
        Ok(())
    }

    fn create_new_leaf(
        &mut self,
        left_page_id: u32,
        split_key: u32,
        right_leaf: BTreeLeaf,
    ) -> Result<(), Error> {
        let mut pager = self.pager.borrow_mut();
        // 右の新しいリーフノードを割り当てる
        let right_page_id = pager.allocate_node(BTreeNode::Leaf(right_leaf))?;
        // 親ノードを取得して更新する
        let parent_page_id = match pager.get_page(left_page_id)? {
            BTreeNode::Leaf(leaf) => leaf.parent.ok_or_else(|| Error::msg("No parent found"))?,
            _ => return Err(Error::msg("Expected leaf node")),
        };
        let parent_node = match pager.get_page(parent_page_id)? {
            BTreeNode::Internal(internal) => internal,
            _ => return Err(Error::msg("Expected internal node")),
        };
        // 親ノードに新しいキーと子を挿入する
        let pos = match parent_node.keys.binary_search(&split_key) {
            Ok(_) => return Err(Error::msg("Duplicate key in internal node")),
            Err(i) => i,
        };
        parent_node.keys.insert(pos, split_key);
        parent_node.children.insert(pos + 1, right_page_id);
        // 左右のリーフノードを更新する
        let mut perv_right_next_leaf = None;
        if let BTreeNode::Leaf(left_leaf) = pager.get_page(left_page_id)? {
            perv_right_next_leaf = left_leaf.next_leaf;
            left_leaf.next_leaf = Some(right_page_id);
        }
        if let BTreeNode::Leaf(right_leaf) = pager.get_page(right_page_id)? {
            right_leaf.parent = Some(parent_page_id);
            right_leaf.next_leaf = perv_right_next_leaf;
        }
        Ok(())
    }

    pub fn select_all(&mut self) -> Result<Vec<Vec<Value>>, Error> {
        let mut results = Vec::new();
        let left_end_node_page_id = self.find_left_end_node_page_id()?;
        // next_leaf を辿りながらすべての行を取得する
        let mut current_leaf_page_id = Some(left_end_node_page_id);
        while let Some(page_id) = current_leaf_page_id {
            let mut pager = self.pager.borrow_mut();
            let leaf_node = match pager.get_page(page_id)? {
                BTreeNode::Leaf(leaf) => leaf,
                _ => return Err(Error::msg("Expected leaf node")),
            };
            for value_bytes in leaf_node.values.iter() {
                results.push(self.schema.deserialize_row_values(value_bytes)?);
            }
            current_leaf_page_id = leaf_node.next_leaf;
        }
        Ok(results)
    }

    pub fn find_left_end_node_page_id(&mut self) -> Result<u32, Error> {
        let mut page_id = self.root_page_id;
        loop {
            let mut pager = self.pager.borrow_mut();
            match pager.get_page(page_id)? {
                BTreeNode::Leaf(_) => return Ok(page_id),
                BTreeNode::Internal(internal) => {
                    page_id = internal.children[0];
                }
            }
        }
    }

    pub fn debug_tree(&mut self) -> Result<(), Error> {
        let mut pager = self.pager.borrow_mut();
        for page_id in 0..pager.num_pages {
            let node = pager.get_page(page_id)?;
            println!("Page ID: {}", page_id);
            match node {
                BTreeNode::Internal(internal) => {
                    println!(
                        "  Internal Node - is_root: {}",
                        page_id == self.root_page_id
                    );
                    println!("  Keys: {:?}", internal.keys);
                    println!("  Children: {:?}", internal.children);
                }
                BTreeNode::Leaf(leaf) => {
                    println!("  Leaf Node - is_root: {}", page_id == self.root_page_id);
                    println!("  Keys: {:?}", leaf.keys);
                    println!("  Values: {:?}", leaf.values);
                    println!("  Next Leaf: {:?}", leaf.next_leaf);
                }
            }
        }
        Ok(())
    }

    fn deserialize_meta_page(buffer: &[u8; PAGE_SIZE]) -> Result<(u32, Schema), Error> {
        let root_page_id = u32::from_le_bytes(buffer[0..4].try_into()?);
        let schema = Schema::deserialize_columns(&buffer[4..])?;
        Ok((root_page_id, schema))
    }

    fn serialize_meta_page(&self) -> [u8; PAGE_SIZE] {
        let mut meta_page = [0u8; PAGE_SIZE];
        meta_page[0..4].copy_from_slice(&self.root_page_id.to_le_bytes());
        let schema_bytes = self.schema.serialize_columns();
        meta_page[4..schema_bytes.len() + 4].copy_from_slice(&schema_bytes);
        meta_page
    }
}
