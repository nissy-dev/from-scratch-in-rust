use std::{fmt, ops};

use crate::code::OpCodes;

#[derive(Debug, Clone, PartialOrd)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum FunctionType {
    Function,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct Function {
    arity: usize,
    name: String,
    pub codes: OpCodes,
}

impl Function {
    pub fn new(name: &str) -> Self {
        Function {
            arity: 0,
            name: name.to_owned(),
            codes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum Object {
    String(String),
    Function(Function),
}

impl Value {
    pub fn is_falsy(&self) -> bool {
        matches!(self, Value::Nil) || matches!(self, Value::Boolean(false))
    }
}

impl std::cmp::PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Object(a), Value::Object(b)) => a == b,
            _ => false,
        }
    }
}

impl std::cmp::Eq for Value {}

impl ops::Add<Value> for Value {
    type Output = Value;

    fn add(self, rhs: Value) -> Self::Output {
        match (&self, &rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Object(Object::String(a)), Value::Object(Object::String(b))) => {
                Value::Object(Object::String(a.to_owned() + b))
            }
            _ => panic!("unsupported operation: {:?} + {:?}", self, rhs),
        }
    }
}

impl ops::Sub<Value> for Value {
    type Output = Value;

    fn sub(self, rhs: Value) -> Self::Output {
        match (&self, &rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
            _ => panic!("unsupported operation: {:?} - {:?}", self, rhs),
        }
    }
}

impl ops::Mul<Value> for Value {
    type Output = Value;

    fn mul(self, rhs: Value) -> Self::Output {
        match (&self, &rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
            _ => panic!("unsupported operation: {:?} * {:?}", self, rhs),
        }
    }
}

impl ops::Div<Value> for Value {
    type Output = Value;

    fn div(self, rhs: Value) -> Self::Output {
        match (&self, &rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a / b),
            _ => panic!("unsupported operation: {:?} / {:?}", self, rhs),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(val) => write!(f, "{}", val),
            Value::Boolean(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "nil"),
            Value::Object(val) => write!(f, "{}", val),
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Object::String(val) => write!(f, "{}", val),
            Object::Function(val) => write!(f, "<fn {}>", val.name),
        }
    }
}
