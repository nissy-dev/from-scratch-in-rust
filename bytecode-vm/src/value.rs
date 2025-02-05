use std::ops;

#[derive(Debug, Clone, PartialOrd)]
pub enum Value {
    Number(f64),
    Boolean(bool),
    Nil,
    Object(Object),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub enum Object {
    String(String),
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
