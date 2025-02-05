use std::{cell::RefCell, rc::Rc};

use crate::{
    compiler::{OpCode, OpCodes},
    token::Location,
    value::{Object, Value},
};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
}

#[derive(Debug)]
pub struct ObjectNode {
    value: Object,
    next: Option<Rc<RefCell<ObjectNode>>>,
}

#[derive(Debug)]
pub struct VirtualMachine {
    codes: OpCodes,
    stack: Vec<Value>,
    pub object_list: Option<Rc<RefCell<ObjectNode>>>,
}

impl VirtualMachine {
    pub fn new(codes: OpCodes) -> Self {
        VirtualMachine {
            codes,
            stack: Vec::with_capacity(STACK_MAX),
            object_list: None,
        }
    }

    pub fn interpret(&mut self) -> Result<(), InterpretError> {
        self.run()
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        while let Some((instruction, loc)) = self.codes.pop_front() {
            match instruction {
                OpCode::Return => self.return_op(&loc)?,
                OpCode::Constant(value) => self.stack_push(value),
                OpCode::Nil => self.stack_push(Value::Nil),
                OpCode::True => self.stack_push(Value::Boolean(true)),
                OpCode::False => self.stack_push(Value::Boolean(false)),
                OpCode::Negate => self.negate_op(&loc)?,
                OpCode::Not => self.not_op(&loc)?,
                OpCode::Add => self.binary_op(|a, b| a + b, &loc)?,
                OpCode::Subtract => self.binary_op(|a, b| a - b, &loc)?,
                OpCode::Multiply => self.binary_op(|a, b| a * b, &loc)?,
                OpCode::Divide => self.binary_op(|a, b| a / b, &loc)?,
                OpCode::Equal => self.binary_op(|a, b| Value::Boolean(a == b), &loc)?,
                OpCode::Greater => self.binary_op(|a, b| Value::Boolean(a > b), &loc)?,
                OpCode::Less => self.binary_op(|a, b| Value::Boolean(a < b), &loc)?,
            }
        }

        Ok(())
    }

    fn return_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.pop() {
            println!("{:?}", value);
            return Ok(());
        }
        self.report_error(loc, "No value to return")
    }

    fn negate_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match self.stack.pop() {
            Some(Value::Number(value)) => {
                self.stack_push(Value::Number(-value));
                Ok(())
            }
            _ => self.report_error(loc, "Operand is not a number value"),
        }
    }

    fn not_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match self.stack.pop() {
            Some(value) => {
                self.stack_push(Value::Boolean(value.is_falsy()));
                Ok(())
            }
            _ => self.report_error(loc, "Operand must have a value"),
        }
    }

    fn binary_op(
        &mut self,
        op: fn(Value, Value) -> Value,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.pop()) {
            (Some(a), Some(b)) => {
                self.stack_push(op(b, a));
                Ok(())
            }
            _ => self.report_error(loc, "Operands must have two values"),
        }
    }

    fn stack_push(&mut self, value: Value) {
        if let Value::Object(object) = value.clone() {
            self.object_list = Some(Rc::new(RefCell::new(ObjectNode {
                value: object,
                next: self.object_list.clone(),
            })));
        }
        self.stack.push(value);
    }

    fn report_error(&self, loc: &Location, message: &str) -> Result<(), InterpretError> {
        tracing::error!(
            "[line {}, col {}] Error: '{}'",
            loc.line,
            loc.column,
            message,
        );
        Err(InterpretError::RuntimeError)
    }
}
