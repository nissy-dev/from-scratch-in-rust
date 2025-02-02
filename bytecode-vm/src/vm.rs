use crate::compiler::{OpCode, OpCodes, Value};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
}

#[derive(Debug)]
pub struct VirtualMachine {
    op_codes: OpCodes,
    stack: Vec<Value>,
}

impl VirtualMachine {
    pub fn new(op_codes: OpCodes) -> Self {
        VirtualMachine {
            op_codes,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn interpret(&mut self) -> Result<(), InterpretError> {
        self.run()
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        while let Some((instruction, _)) = self.op_codes.pop_front() {
            match instruction {
                OpCode::Return => {
                    let value = self.stack.pop().ok_or(InterpretError::RuntimeError)?;
                    println!("{:?}", value);
                    return Ok(());
                }
                OpCode::Constant(value) => {
                    self.stack.push(value);
                }
                OpCode::Negate => {
                    let value = self.stack.pop().ok_or(InterpretError::RuntimeError)?;
                    let Value::Number(value) = value;
                    self.stack.push(Value::Number(-value));
                }
                OpCode::Add => {
                    self.binary_op(|a, b| a + b)?;
                }
                OpCode::Subtract => {
                    self.binary_op(|a, b| b - a)?;
                }
                OpCode::Multiply => {
                    self.binary_op(|a, b| a * b)?;
                }
                OpCode::Divide => {
                    self.binary_op(|a, b| a / b)?;
                }
            }
        }

        Ok(())
    }

    fn binary_op(&mut self, op: fn(f64, f64) -> f64) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.pop()) {
            (Some(Value::Number(a)), Some(Value::Number(b))) => {
                self.stack.push(Value::Number(op(a, b)));
                Ok(())
            }
            _ => {
                tracing::error!("Expected two numbers on stack");
                Err(InterpretError::RuntimeError)
            }
        }
    }
}
