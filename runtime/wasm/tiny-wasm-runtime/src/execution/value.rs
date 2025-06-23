#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    I32(i32),
    I64(i64),
}

impl From<Value> for i32 {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(value) => value,
            _ => panic!("type mismatch"),
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::I32(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::I64(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::I32(if value { 1 } else { 0 })
    }
}

impl std::ops::Add for Value {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::I32(lhs), Value::I32(rhs)) => Value::I32(lhs + rhs),
            (Value::I64(lhs), Value::I64(rhs)) => Value::I64(lhs + rhs),
            _ => panic!("type mismatch"),
        }
    }
}

impl std::ops::Sub for Value {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::I32(lhs), Value::I32(rhs)) => Value::I32(lhs - rhs),
            (Value::I64(lhs), Value::I64(rhs)) => Value::I64(lhs - rhs),
            _ => panic!("type mismatch"),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::I32(lhs), Value::I32(rhs)) => lhs.partial_cmp(rhs),
            (Value::I64(lhs), Value::I64(rhs)) => lhs.partial_cmp(rhs),
            _ => panic!("type mismatch"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelKind {
    If,
}

pub struct Label {
    pub kind: LabelKind,
    pub pc: usize,
    pub sp: usize,
    pub arity: usize,
}
