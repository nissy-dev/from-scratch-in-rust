// BTree binary layout
// +-----------------------------+ ← offset 0
// | btype (2B)                 |
// | nkeys (2B)                 |
// +-----------------------------+
// | ptr[0] (8B)                |
// | ptr[1] (8B)                |
// | ...                        |
// +-----------------------------+
// | offset[1] (2B)             |
// | offset[2] (2B)             |
// | ...                        |
// +-----------------------------+
// |   free space               |
// +-----------------------------+
// | key/val entries (後ろ詰め)  |
// +-----------------------------+ ← offset 4096
//
// key/val entries layout
// +-----------------------------+
// | key_len (2B)               |
// | val_len (2B)               |
// | key (variable)             |
// | val (variable)             |

use std::cmp::Ordering;

const BTREE_PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone)]
pub struct BNode {
    buf: Vec<u8>,
}

impl BNode {
    const BTYPE_NODE: u16 = 0x0001;
    const BTYPE_LEAF: u16 = 0x0002;

    pub fn new(page_size: usize) -> Self {
        Self {
            buf: vec![0u8; page_size],
        }
    }

    fn b_type(&self) -> u16 {
        u16::from_le_bytes([self.buf[0], self.buf[1]])
    }

    fn n_keys(&self) -> u16 {
        u16::from_le_bytes([self.buf[2], self.buf[3]])
    }

    fn set_header(&mut self, b_type: u16, n_keys: u16) {
        self.buf[0..2].copy_from_slice(&b_type.to_le_bytes());
        self.buf[2..4].copy_from_slice(&n_keys.to_le_bytes());
    }

    fn n_bytes(&self) -> u16 {
        self.kv_pos(self.n_keys())
    }

    pub fn get_ptr(&self, idx: u16) -> u64 {
        if idx >= self.n_keys() {
            panic!("Index out of bounds: {} >= {}", idx, self.n_keys());
        }
        let pos = 4 + 8 * idx as usize;
        let ptr_bytes: [u8; 8] = self.buf[pos..pos + 8]
            .try_into()
            .expect("ptr bytes conversion failed");
        u64::from_le_bytes(ptr_bytes)
    }

    fn set_ptr(&mut self, idx: u16, val: u64) {
        if idx >= self.n_keys() {
            panic!("Index out of bounds: {} >= {}", idx, self.n_keys());
        }
        let pos = 4 + 8 * idx as usize;
        let bytes: [u8; 8] = val.to_le_bytes();
        self.buf[pos..pos + 8].copy_from_slice(&bytes);
    }

    fn get_offset(&self, idx: u16) -> u16 {
        if idx == 0 {
            return 0;
        }
        let pos = (4 + 8 * self.n_keys() + 2 * (idx - 1)) as usize;
        let offset_bytes: [u8; 2] = self.buf[pos..pos + 2]
            .try_into()
            .expect("offset bytes conversion failed");
        u16::from_le_bytes(offset_bytes)
    }

    fn set_offset(&mut self, idx: u16, offset: u16) {
        if idx == 0 {
            panic!("Cannot set offset for index 0");
        }
        if idx > self.n_keys() {
            panic!("Index out of bounds: {} > {}", idx, self.n_keys());
        }
        let pos = (4 + 8 * self.n_keys() + 2 * (idx - 1)) as usize;
        let bytes: [u8; 2] = offset.to_le_bytes();
        self.buf[pos..pos + 2].copy_from_slice(&bytes);
    }

    fn kv_pos(&self, idx: u16) -> u16 {
        if idx > self.n_keys() {
            panic!("Index out of bounds: {} > {}", idx, self.n_keys());
        }
        let offset = self.get_offset(idx);
        4 + 8 * self.n_keys() + 2 * self.n_keys() + offset
    }

    fn get_key(&self, idx: u16) -> &[u8] {
        if idx >= self.n_keys() {
            panic!("Index out of bounds: {} >= {}", idx, self.n_keys());
        }
        let pos = self.kv_pos(idx) as usize;
        let key_len_bytes: [u8; 2] = self.buf[pos..pos + 2]
            .try_into()
            .expect("key length bytes conversion failed");
        let key_len = u16::from_le_bytes(key_len_bytes) as usize;
        let start = pos + 4;
        let end = start + key_len;
        &self.buf[start..end]
    }

    fn get_val(&self, idx: u16) -> &[u8] {
        if idx >= self.n_keys() {
            panic!("Index out of bounds: {} >= {}", idx, self.n_keys());
        }
        let pos = self.kv_pos(idx) as usize;
        let key_len_bytes: [u8; 2] = self.buf[pos..pos + 2]
            .try_into()
            .expect("key length bytes conversion failed");
        let key_len = u16::from_le_bytes(key_len_bytes) as usize;
        let val_len_bytes: [u8; 2] = self.buf[pos + 2..pos + 4]
            .try_into()
            .expect("value length bytes conversion failed");
        let val_len = u16::from_le_bytes(val_len_bytes) as usize;
        let start = pos + 4 + key_len;
        let end = start + val_len;
        &self.buf[start..end]
    }

    fn append(&mut self, idx: u16, ptr: u64, key: &[u8], val: &[u8]) {
        self.set_ptr(idx, ptr);

        let pos = self.kv_pos(idx) as usize;
        let k_len = key.len();
        let v_len = val.len();

        // キーとバリューの長さを書き込み
        self.buf[pos..pos + 2].copy_from_slice(&k_len.to_le_bytes());
        self.buf[pos + 2..pos + 4].copy_from_slice(&v_len.to_le_bytes());

        // キー本体を書き込み
        let val_start = pos + 4;
        self.buf[val_start..val_start + k_len].copy_from_slice(key);
        self.buf[val_start + k_len..val_start + k_len + v_len].copy_from_slice(val);

        // 次のキーのオフセット更新
        let current_offset = self.get_offset(idx);
        let new_offset = current_offset + 4 + k_len as u16 + v_len as u16;
        self.set_offset(idx + 1, new_offset);
    }

    // prev_node の特定の idx から始まる n 個のキーとバリューを、self の 特定の idx の後に追加する
    fn append_range(&mut self, prev_node: &Self, dst_idx: u16, src_idx: u16, n: u16) {
        for i in 0..n {
            let s_idx = src_idx + i;
            let d_idx = dst_idx + i;
            let ptr = prev_node.get_ptr(s_idx);
            let key = prev_node.get_key(s_idx);
            let val = prev_node.get_val(s_idx);
            self.append(d_idx, ptr, key, val);
        }
    }

    fn leaf_insert(&mut self, prev_node: &Self, idx: u16, key: &[u8], val: &[u8]) {
        self.set_header(Self::BTYPE_LEAF, prev_node.n_keys() + 1);
        // idx 前までのキーとバリューをコピー
        self.append_range(prev_node, 0, 0, idx);
        self.append(idx, 0, key, val);
        // idx 以降のキーとバリューをコピー
        self.append_range(prev_node, idx + 1, idx, prev_node.n_keys() - idx);
    }

    fn leaf_update(&mut self, prev_node: &Self, idx: u16, key: &[u8], val: &[u8]) {
        self.set_header(Self::BTYPE_LEAF, prev_node.n_keys());
        // idx 前までのキーとバリューをコピー
        self.append_range(prev_node, 0, 0, idx);
        self.append(idx, 0, key, val);
        // idx より後のキーとバリューをコピー
        self.append_range(prev_node, idx + 1, idx + 1, prev_node.n_keys() - idx - 1);
    }

    // key に等しいかそれ以下のキーのインデックスを返す
    // key はソートされていることを前提とする
    fn lookup_le(&self, key: &[u8]) -> u16 {
        let n_keys = self.n_keys();
        let mut idx: u16 = 0;
        while idx < n_keys {
            match self.get_key(idx).cmp(key) {
                Ordering::Equal => return idx,
                Ordering::Greater => return idx.saturating_sub(1),
                Ordering::Less => { /* fallthrough */ }
            }
            idx += 1;
        }
        return idx.saturating_sub(1);
    }

    fn resize(&mut self, new_size: usize) {
        if new_size > self.buf.len() {
            self.buf.resize(new_size, 0);
        } else {
            self.buf.truncate(new_size);
        }
    }
}

pub trait PageStore {
    fn new(node: &BNode) -> u64;
    fn get(&self, ptr: u64) -> BNode;
    fn del(&mut self, ptr: u64);
}

pub struct BTree<S: PageStore> {
    root: u64,
    store: S,
}

impl<S: PageStore> BTree<S> {
    pub fn new(store: S) -> Self {
        Self { root: 0, store }
    }

    pub fn insert(&mut self, key: &[u8], val: &[u8]) {
        if self.root == 0 {
            // 初回挿入時は新しいルートノードを作成
            let mut root_node = BNode::new(BTREE_PAGE_SIZE);
            root_node.set_header(BNode::BTYPE_LEAF, 1);
            root_node.leaf_insert(&root_node, 0, key, val);
            self.root = S::new(&root_node);
            self.store.del(self.root); // ルートノードは新規作成なので削除
        } else {
            // 既存のルートノードに挿入
            let mut root_node = self.store.get(self.root);
            let new_node = self.tree_insert(&root_node, key, val);
            if new_node.n_keys() > 0 {
                // 新しいノードが返された場合、ルートを更新
                self.root = S::new(&new_node);
                self.store.del(self.root); // 古いルートノードを削除
            }
        }
    }

    fn tree_insert(&mut self, node: &BNode, key: &[u8], val: &[u8]) -> BNode {
        let mut new_node = BNode::new(BTREE_PAGE_SIZE * 2);
        let idx = node.lookup_le(key);
        match node.b_type() {
            BNode::BTYPE_LEAF => {
                if node.get_key(idx) == key {
                    new_node.leaf_update(node, idx, key, val);
                } else {
                    new_node.leaf_insert(node, idx + 1, key, val);
                }
            }
            BNode::BTYPE_NODE => {
                // recursive insertion to the kid node
                let kid_ptr = node.get_ptr(idx);
                let kid_node = self.tree_insert(&self.store.get(kid_ptr), key, val);
                // after insertion, split the result
                let splitted_nodes = self.node_split3(&kid_node);
                self.node_replace_kid_nodes(&mut new_node, node, idx, &splitted_nodes);
                // deallocate the old kid node
                self.store.del(kid_ptr);
            }
            _ => panic!("Unknown node type"),
        }
        return new_node;
    }

    fn node_split2(&self, left_node: &mut BNode, right_node: &mut BNode, prev_node: &BNode) {
        assert!(prev_node.n_keys() >= 2);
        let mut n_left = prev_node.n_keys() / 2;

        let left_bytes = |n_left: u16, prev_node: &BNode| {
            (4 + 8 * n_left + 2 * n_left + prev_node.get_offset(n_left)) as usize
        };
        while left_bytes(n_left, prev_node) > BTREE_PAGE_SIZE {
            n_left -= 1;
        }
        assert!(n_left >= 1);

        let right_bytes = |n_left: u16, prev_node: &BNode| {
            prev_node.n_bytes() as usize - left_bytes(n_left, prev_node) + 4
        };
        while right_bytes(n_left, prev_node) > BTREE_PAGE_SIZE {
            n_left += 1;
        }
        assert!(n_left < prev_node.n_keys());
        let n_right = prev_node.n_keys() - n_left;

        // new nodes
        left_node.set_header(prev_node.b_type(), n_left);
        right_node.set_header(prev_node.b_type(), n_right);
        left_node.append_range(prev_node, 0, 0, n_left);
        right_node.append_range(prev_node, 0, n_left, n_right);

        assert!(right_node.n_bytes() as usize <= BTREE_PAGE_SIZE);
    }

    fn node_split3(&self, prev_node: &BNode) -> Vec<BNode> {
        if prev_node.n_bytes() as usize <= BTREE_PAGE_SIZE {
            let mut node = prev_node.clone();
            node.resize(BTREE_PAGE_SIZE);
            return vec![node];
        }

        // まずは 2 つのノードに分割
        let mut left_node = BNode::new(2 * BTREE_PAGE_SIZE);
        let mut right_node = BNode::new(BTREE_PAGE_SIZE);
        self.node_split2(&mut left_node, &mut right_node, prev_node);
        if left_node.n_bytes() as usize <= BTREE_PAGE_SIZE {
            left_node.resize(BTREE_PAGE_SIZE);
            return vec![left_node, right_node];
        }
        // さらに left_node を２つに分割
        let mut new_left_node = BNode::new(BTREE_PAGE_SIZE);
        let mut middle = BNode::new(BTREE_PAGE_SIZE);
        self.node_split2(&mut new_left_node, &mut middle, &left_node);
        assert!(new_left_node.n_bytes() as usize <= BTREE_PAGE_SIZE);
        vec![new_left_node, middle, right_node]
    }

    fn node_replace_kid_nodes(
        &mut self,
        new_node: &mut BNode,
        old_node: &BNode,
        idx: u16,
        kids: &Vec<BNode>,
    ) {
        let inc = kids.len() as u16;
        new_node.set_header(BNode::BTYPE_NODE, old_node.n_keys() + inc - 1);
        new_node.append_range(old_node, 0, 0, idx);
        for (i, node) in kids.iter().enumerate() {
            new_node.append(idx + i as u16, S::new(node), node.get_key(0), &[]);
        }
        new_node.append_range(old_node, idx + inc, idx + 1, old_node.n_keys() - (idx + 1));
    }
}
