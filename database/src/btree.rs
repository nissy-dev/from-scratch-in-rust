// BTree binary layout
// +-----------------------------+ ← offset 0
// | btype (2B)                 |
// | nkeys (2B)                 |
// +-----------------------------+
// | ptr[0] (8B)                |
// | ptr[1] (8B)                |
// | ...                       |
// +-----------------------------+
// | offset[1] (2B)             |
// | offset[2] (2B)             |
// | ...                       |
// +-----------------------------+
// |   free space              |
// +-----------------------------+
// | key/val entries (後ろ詰め) |
// +-----------------------------+ ← offset 4096
//
// key/val entries layout
// +-----------------------------+
// | key_len (2B)              |
// | val_len (2B)              |
// | key (variable)            |
// | val (variable)            |

const BTYPE_NODE: u16 = 0x0001;
const BTYPE_LEAF: u16 = 0x0002;

pub struct BNode {
    buf: Vec<u8>,
}

impl BNode {
    pub fn new() -> Self {
        Self {
            buf: vec![0u8; 4096],
        }
    }

    pub fn set_header(&mut self, b_type: u16, n_keys: u16) {
        self.buf[0..2].copy_from_slice(&b_type.to_le_bytes());
        self.buf[2..4].copy_from_slice(&n_keys.to_le_bytes());
    }

    pub fn b_type(&self) -> u16 {
        u16::from_le_bytes([self.buf[0], self.buf[1]])
    }

    pub fn n_keys(&self) -> u16 {
        u16::from_le_bytes([self.buf[2], self.buf[3]])
    }

    // read the child pointers
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

    // write the child pointers
    pub fn set_ptr(&mut self, idx: u16, val: u64) {
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

    fn kv_pos(&self, idx: u16) -> u16 {
        if idx > self.n_keys() {
            panic!("Index out of bounds: {} > {}", idx, self.n_keys());
        }
        let offset = self.get_offset(idx);
        4 + 8 * self.n_keys() + 2 * self.n_keys() + offset
    }

    pub fn get_key(&self, idx: u16) -> &[u8] {
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

    pub fn get_val(&self, idx: u16) -> &[u8] {
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
}
