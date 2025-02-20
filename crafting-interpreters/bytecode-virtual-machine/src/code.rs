use crate::{
    compiler::UpValue,
    token::Location,
    value::{Object, Value},
};

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
    Call(usize),
    Closure(Object, Vec<UpValue>),
    GetUpValue(usize),
    SetUpValue(usize),
}

pub type OpCodes = Vec<(OpCode, Location)>;
