use std::fmt;

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

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpCode::Return => write!(f, "Return"),
            OpCode::Constant(value) => write!(f, "Constant({})", value),
            OpCode::Negate => write!(f, "Negate"),
            OpCode::Add => write!(f, "Add"),
            OpCode::Subtract => write!(f, "Subtract"),
            OpCode::Multiply => write!(f, "Multiply"),
            OpCode::Divide => write!(f, "Divide"),
            OpCode::Nil => write!(f, "Nil"),
            OpCode::True => write!(f, "True"),
            OpCode::False => write!(f, "False"),
            OpCode::Not => write!(f, "Not"),
            OpCode::Equal => write!(f, "Equal"),
            OpCode::Greater => write!(f, "Greater"),
            OpCode::Less => write!(f, "Less"),
            OpCode::Print => write!(f, "Print"),
            OpCode::Pop => write!(f, "Pop"),
            OpCode::DefineGlobal => write!(f, "DefineGlobal"),
            OpCode::GetGlobal => write!(f, "GetGlobal"),
            OpCode::SetGlobal => write!(f, "SetGlobal"),
            OpCode::GetLocal(slot) => write!(f, "GetLocal({})", slot),
            OpCode::SetLocal(slot) => write!(f, "SetLocal({})", slot),
            OpCode::JumpIfFalse(offset) => write!(f, "JumpIfFalse({})", offset),
            OpCode::Jump(offset) => write!(f, "Jump({})", offset),
            OpCode::Loop(offset) => write!(f, "Loop({})", offset),
            OpCode::Call(arg_count) => write!(f, "Call({})", arg_count),
            OpCode::Closure(_, _) => write!(f, "Closure"),
            OpCode::GetUpValue(slot) => write!(f, "GetUpValue({})", slot),
            OpCode::SetUpValue(slot) => write!(f, "SetUpValue({})", slot),
        }
    }
}
