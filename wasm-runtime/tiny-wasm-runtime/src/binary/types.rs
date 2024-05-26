#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    I32, // 0x7F
    I64, // 0x7E
}

impl From<u8> for ValueType {
    fn from(value: u8) -> Self {
        match value {
            0x7F => ValueType::I32,
            0x7E => ValueType::I64,
            _ => panic!("invalid value type: {:X}", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLocal {
    pub type_count: u32,
    pub value_type: ValueType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportDesc {
    Func(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Export {
    pub name: String,
    pub desc: ExportDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportDesc {
    Func(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Import {
    pub module: String,
    pub field: String,
    pub desc: ImportDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
    // データの配置先のメモリのインデックス
    // version 1 ではメモリは 1 つしか扱えないので、常に 0 になる
    pub memory_idx: u32,
    pub offset: u32,
    pub init: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub block_type: BlockType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    ValueType(Vec<ValueType>),
    Void,
}

impl BlockType {
    pub fn result_count(&self) -> usize {
        match self {
            BlockType::ValueType(value_types) => value_types.len(),
            BlockType::Void => 0,
        }
    }
}
