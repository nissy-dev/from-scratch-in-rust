use crate::{token::Location, value::Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum OpCode {
    Return,
    Constant(Value),
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Nil,
    True,
    False,
    Not,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal(usize),
    SetLocal(usize),
    JumpIfFalse(usize),
    Jump(usize),
    Loop(usize),
}

// struct Chunk {
//     codes: Vec<(OpCode, Location)>,
//     constants: Vec<Value>,
// }

// impl Chunk {
//     fn new() -> Self {
//         Chunk {
//             codes: Vec::new(),
//             constants: Vec::new(),
//         }
//     }

//     fn write(&mut self, code: OpCode, location: Location) {
//         self.codes.push((code, location));
//     }

//     fn add_constant(&mut self, value: Value) -> usize {
//         self.constants.push(value);
//         self.constants.len() - 1
//     }
// }

pub type OpCodes = Vec<(OpCode, Location)>;
