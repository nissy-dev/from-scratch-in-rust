# B+Tree実装 - 現在のコード

## backend.rs

```rust
// filepath: [backend.rs](http://_vscodecontentref_/0)
use std::{
    collections::{hash_map::Entry, HashMap},
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

    fn insert_child(&mut self, key: u32, child_page_id: u32) -> Result<()> {
        let pos = self.keys.binary_search(&key).unwrap_or_else(|e| e);
        self.keys.insert(pos, key);
        self.children.insert(pos + 1, child_page_id);
        Ok(())
    }
}

#[derive(Debug)]
struct BTreeLeaf {
    is_root: bool,
    parent: Option<u32>,
    keys: Vec<u32>,
    values: Vec<Vec<u8>>,
    next_leaf: Option<u32>,
}

impl BTreeLeaf {
    fn new(is_root: bool, parent: Option<u32>) -> Self {
        Self {
            is_root,
            parent,
            keys: Vec::new(),
            values: Vec::new(),
            next_leaf: None,
        }
    }

    fn serialize(&self) -> [u8; PAGE_SIZE] {
        let mut buffer = [0u8; PAGE_SIZE];
        buffer[0] = 1;
        buffer[1] = self.is_root as u8;
        let parent_pointer = self.parent.unwrap_or(PARENT_POINTER_NONE);
        buffer[2..6].copy_from_slice(&parent_pointer.to_le_bytes());
        let next_leaf_pointer = self.next_leaf.unwrap_or(PARENT_POINTER_NONE);
        buffer[6..10].copy_from_slice(&next_leaf_pointer.to_le_bytes());
        buffer[10..14].copy_from_slice(&(self.keys.len() as u32).to_le_bytes());
        let mut offset = 14;
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
        let is_root = buffer[1] != 0;
        let parent = u32::from_le_bytes(buffer[2..6].try_into().unwrap());
        let parent = if parent == PARENT_POINTER_NONE {
            None
        } else {
            Some(parent)
        };
        let next_leaf = u32::from_le_bytes(buffer[6..10].try_into().unwrap());
        let next_leaf = if next_leaf == PARENT_POINTER_NONE {
            None
        } else {
            Some(next_leaf)
        };
        let num_cells = u32::from_le_bytes(buffer[10..14].try_into().unwrap());
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut offset = 14;
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
            is_root,
            parent,
            keys,
            values,
            next_leaf,
        }
    }

    fn can_hold(&self, value: &[u8]) -> bool {
        let used_space: usize = 14
            + self
                .values
                .iter()
                .map(|v| 4 + 4 + v.len()) // key(4) + value_len(4) + value
                .sum::<usize>();
        used_space + 4 + 4 + value.len() <= PAGE_SIZE
    }

    fn insert(&mut self, key: u32, value: Vec<u8>) -> Result<()> {
        if self.keys.contains(&key) {
            return Err(Error::msg("Duplicate key"));
        }
        if !self.can_hold(&value) {
            return Err(Error::msg("Leaf node is full"));
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
        let new_leaf = BTreeLeaf {
            is_root: false,
            parent: self.parent,
            keys: new_keys,
            values: new_values,
            next_leaf: None,
        };
        (split_key, new_leaf)
    }
}

impl BTreeNode {
    fn new_root() -> Self {
        BTreeNode::Leaf(BTreeLeaf::new(true, None))
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

    fn set_parent(&mut self, parent: Option<u32>) {
        match self {
            BTreeNode::Internal(internal) => internal.parent = parent,
            BTreeNode::Leaf(leaf) => leaf.parent = parent,
        }
    }

    fn set_root(&mut self, is_root: bool) {
        match self {
            BTreeNode::Internal(internal) => internal.is_root = is_root,
            BTreeNode::Leaf(leaf) => leaf.is_root = is_root,
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

    fn insert_recursive(&mut self, page_id: u32, key: u32, value: Vec<u8>) -> Result<(), Error> {
        let node = self.pager.get_page(page_id)?;
        match node {
            BTreeNode::Leaf(leaf) => {
                if leaf.can_hold(&value) {
                    leaf.insert(key, value)?;
                    Ok(())
                } else {
                    // ノードを分割する
                    let (split_key, new_leaf) = leaf.split();
                    let new_page_id = self.pager.allocate_node(BTreeNode::Leaf(new_leaf))?;
                    leaf.set_root(false);
                    new_leaf.set_root(true);
                    self.pager.update_node(page_id, leaf)?;
                    self.pager.update_node(new_page_id, new_leaf)?;
                    self.insert_recursive(leaf.parent(), split_key, vec![])?;
                    Ok(())
                }
            }
            BTreeNode::Internal(internal) => {
                let child_page_id = internal.find_child_page_id(key);
                self.insert_recursive(child_page_id, key, value)
            }
        }
    }

    pub fn select_all(&mut self) -> Result<Vec<Vec<Value>>, Error> {
        let mut results = Vec::new();
        let node = self.pager.get_page(Self::BTREE_ROOT_PAGE_ID)?;
        // TODO: 本当はツリー全体を捜査して、すべてのリーフノードの値を取得する必要がある
        let leaf_node = match node {
            BTreeNode::Leaf(leaf) => leaf,
            _ => todo!("Implement selection for non-leaf nodes"),
        };
        for value_bytes in leaf_node.values.iter() {
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
```

## 現在の状況と次のステップ

### 実装済み

- 基本的なBTreeNodeの構造（Internal/Leaf）
- ページのシリアライゼーション/デシリアライゼーション
- リーフノードの分割機能（splitメソッド）
- 単純な挿入機能（ルートがリーフの場合のみ）

### TODO（次に実装が必要な箇所）

- insert_recursiveメソッドの完成
- 内部ノードを辿って適切なリーフノードを見つける機能
- ルート分割時の新しいルート作成
- 内部ノードの分割機能
- B+Treeのリーフノード間リンクの適切な管理
- 完全なツリー走査機能

### コンパイルエラー箇所

- insert_recursiveメソッド内の存在しないメソッド呼び出し
- Pagerのupdate_nodeメソッドが未実装
- BTreeInternalのfind_child_page_idメソッドが未実装
