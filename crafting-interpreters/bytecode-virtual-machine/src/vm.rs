use crate::{
    code::{OpCode, OpCodes},
    table::Table,
    token::Location,
    value::{Object, Value},
};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
}

// #[derive(Debug)]
// pub struct ObjectNode {
//     value: Object,
//     next: Option<Rc<RefCell<ObjectNode>>>,
// }

#[derive(Debug)]
pub struct VirtualMachine {
    codes: OpCodes,
    cursor: usize,
    stack: Vec<Value>,
    globals: Table,
    // pub object_list: Option<Rc<RefCell<ObjectNode>>>,
}

impl VirtualMachine {
    pub fn new(codes: OpCodes) -> Self {
        VirtualMachine {
            codes,
            cursor: 0,
            stack: Vec::with_capacity(STACK_MAX),
            globals: Table::new(30),
            // object_list: None,
        }
    }

    pub fn interpret(&mut self) -> Result<(), InterpretError> {
        while let Some((instruction, loc)) = self.codes.get(self.cursor).cloned() {
            self.cursor += 1;
            match instruction {
                OpCode::Return => self.return_op(&loc)?,
                OpCode::Constant(value) => self.stack_push(value.clone()),
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
                OpCode::Print => self.print_op(&loc)?,
                OpCode::Pop => self.pop_op(&loc)?,
                OpCode::DefineGlobal => self.define_global_op(&loc)?,
                OpCode::GetGlobal => self.get_global_op(&loc)?,
                OpCode::SetGlobal => self.set_global_op(&loc)?,
                OpCode::GetLocal(slot) => self.get_local_op(slot, &loc)?,
                OpCode::SetLocal(slot) => self.set_local_op(slot, &loc)?,
                OpCode::JumpIfFalse(jump_offset) => self.jump_if_false_op(jump_offset, &loc)?,
                OpCode::Jump(jump_offset) => self.jump_op(jump_offset, &loc)?,
                OpCode::Loop(loop_offset) => self.loop_op(loop_offset, &loc)?,
            }
        }

        Ok(())
    }

    fn return_op(&mut self, _loc: &Location) -> Result<(), InterpretError> {
        Ok(())
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

    fn print_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.pop() {
            println!("{}", value);
            Ok(())
        } else {
            self.report_error(loc, "No value to print")
        }
    }

    fn pop_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        if self.stack.is_empty() {
            self.report_error(loc, "No value to pop")
        } else {
            self.stack.pop();
            Ok(())
        }
    }

    fn define_global_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.pop()) {
            (Some(value), Some(Value::Object(Object::String(key)))) => {
                self.globals.set(&key, value);
                Ok(())
            }
            _ => self.report_error(loc, "No valid values to define"),
        }
    }

    fn get_global_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match self.stack.pop() {
            Some(Value::Object(Object::String(key))) => {
                if let Some(value) = self.globals.get(&key) {
                    self.stack_push(value.clone());
                    Ok(())
                } else {
                    self.report_error(loc, "Undefined global variable")
                }
            }
            _ => self.report_error(loc, "No valid values to get"),
        }
    }

    fn set_global_op(&mut self, loc: &Location) -> Result<(), InterpretError> {
        match (self.stack.pop(), self.stack.last()) {
            (Some(value), Some(Value::Object(Object::String(key)))) => {
                if self.globals.get(&key).is_none() {
                    self.report_error(loc, "Undefined global variable")
                } else {
                    self.globals.set(&key, value);
                    Ok(())
                }
            }
            _ => self.report_error(loc, "No valid values to set"),
        }
    }

    fn get_local_op(&mut self, slot: usize, loc: &Location) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.get(slot) {
            self.stack_push(value.clone());
            Ok(())
        } else {
            self.report_error(loc, "Undefined local variable")
        }
    }

    fn set_local_op(&mut self, slot: usize, loc: &Location) -> Result<(), InterpretError> {
        match (self.stack.last(), self.stack.get(slot)) {
            (Some(value), Some(_)) => {
                self.stack[slot] = value.clone();
                Ok(())
            }
            _ => self.report_error(loc, "No valid values to set"),
        }
    }

    fn jump_if_false_op(
        &mut self,
        jump_offset: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.last() {
            if value.is_falsy() {
                self.cursor += jump_offset;
            }
            Ok(())
        } else {
            self.report_error(loc, "No value to if else condition")
        }
    }

    fn jump_op(&mut self, jump_offset: usize, _loc: &Location) -> Result<(), InterpretError> {
        self.cursor += jump_offset;
        Ok(())
    }

    fn loop_op(&mut self, loop_offset: usize, _loc: &Location) -> Result<(), InterpretError> {
        self.cursor -= loop_offset;
        Ok(())
    }

    fn stack_push(&mut self, value: Value) {
        // if let Value::Object(object) = value.clone() {
        //     self.object_list = Some(Rc::new(RefCell::new(ObjectNode {
        //         value: object,
        //         next: self.object_list.clone(),
        //     })));
        // }
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
