use crate::{
    compiler::{OpCode, OpCodes},
    token::Location,
    value::Value,
};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
}

#[derive(Debug)]
pub struct VirtualMachine {
    codes: OpCodes,
    stack: Vec<Value>,
}

impl VirtualMachine {
    pub fn new(codes: OpCodes) -> Self {
        VirtualMachine {
            codes,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn interpret(&mut self) -> Result<(), InterpretError> {
        self.run()
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        while let Some((instruction, loc)) = self.codes.pop_front() {
            match instruction {
                OpCode::Return => self.return_op(&loc)?,
                OpCode::Constant(value) => self.stack.push(value),
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Boolean(true)),
                OpCode::False => self.stack.push(Value::Boolean(false)),
                OpCode::Negate => self.negate_op(&loc)?,
                OpCode::Not => self.not_op(&loc)?,
                OpCode::Add => self.binary_op(|a, b| a + b, &loc)?,
                OpCode::Subtract => self.binary_op(|a, b| b - a, &loc)?,
                OpCode::Multiply => self.binary_op(|a, b| a * b, &loc)?,
                OpCode::Divide => self.binary_op(|a, b| b / a, &loc)?,
                OpCode::Equal => self.binary_op(|a, b| Value::Boolean(a == b), &loc)?,
                OpCode::Greater => self.binary_op(|a, b| Value::Boolean(b > a), &loc)?,
                OpCode::Less => self.binary_op(|a, b| Value::Boolean(b < a), &loc)?,
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
                self.stack.push(Value::Number(-value));
                Ok(())
            }
            _ => self.report_error(loc, "Operand is not a number value"),
        }
    }

    fn not_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match self.stack.pop() {
            Some(value) => {
                self.stack.push(Value::Boolean(value.is_falsy()));
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
                self.stack.push(op(a, b));
                Ok(())
            }
            _ => self.report_error(loc, "Operands must be numbers"),
        }
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
