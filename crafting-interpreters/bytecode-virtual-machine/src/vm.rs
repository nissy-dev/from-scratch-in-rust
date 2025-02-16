use crate::{
    code::OpCode,
    compiler::{CompileError, Compiler},
    table::Table,
    token::Location,
    value::{Function, Object, Value},
};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
    CompileError(CompileError),
}

impl From<CompileError> for InterpretError {
    fn from(error: CompileError) -> Self {
        InterpretError::CompileError(error)
    }
}

#[derive(Debug, Clone)]
struct CallFrame {
    function: Function,
    ip: usize,
    slots: Vec<Value>,
}

impl CallFrame {
    fn new(function: Function, slots: Vec<Value>) -> Self {
        CallFrame {
            function,
            ip: 0,
            slots,
        }
    }
}

// #[derive(Debug)]
// pub struct ObjectNode {
//     value: Object,
//     next: Option<Rc<RefCell<ObjectNode>>>,
// }

#[derive(Debug)]
pub struct VirtualMachine {
    stack: Vec<Value>,
    globals: Table,
    frames: Vec<CallFrame>,
    // pub object_list: Option<Rc<RefCell<ObjectNode>>>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            stack: Vec::with_capacity(STACK_MAX),
            globals: Table::new(30),
            frames: Vec::new(),
            // object_list: None,
        }
    }

    pub fn interpret(&mut self, source: String) -> Result<(), InterpretError> {
        let mut compiler = Compiler::new(source);
        let function = compiler.compile()?;
        self.frames
            .push(CallFrame::new(function, self.stack.clone()));
        self.run()
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        if let Some(mut frame) = self.frames.pop() {
            while frame.ip < frame.function.codes.len() {
                let (instruction, loc) = frame.function.codes[frame.ip].clone();
                // println!("instruction: {:?}", instruction);
                self.execute_operation(instruction, &mut frame, &loc)?;
                frame.ip += 1;
                // println!("stack: {:?}", frame.slots);
            }
            return Ok(());
        }
        tracing::error!("No frames to run");
        Err(InterpretError::RuntimeError)
    }

    fn execute_operation(
        &mut self,
        instruction: OpCode,
        frame: &mut CallFrame,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match instruction {
            OpCode::Return => self.return_op(loc)?,
            OpCode::Constant(value) => self.stack_push(frame, value.clone()),
            OpCode::Nil => self.stack_push(frame, Value::Nil),
            OpCode::True => self.stack_push(frame, Value::Boolean(true)),
            OpCode::False => self.stack_push(frame, Value::Boolean(false)),
            OpCode::Negate => self.negate_op(frame, loc)?,
            OpCode::Not => self.not_op(frame, loc)?,
            OpCode::Add => self.binary_op(frame, |a, b| a + b, loc)?,
            OpCode::Subtract => self.binary_op(frame, |a, b| a - b, loc)?,
            OpCode::Multiply => self.binary_op(frame, |a, b| a * b, loc)?,
            OpCode::Divide => self.binary_op(frame, |a, b| a / b, loc)?,
            OpCode::Equal => self.binary_op(frame, |a, b| Value::Boolean(a == b), loc)?,
            OpCode::Greater => self.binary_op(frame, |a, b| Value::Boolean(a > b), loc)?,
            OpCode::Less => self.binary_op(frame, |a, b| Value::Boolean(a < b), loc)?,
            OpCode::Print => self.print_op(frame, loc)?,
            OpCode::Pop => self.pop_op(frame, loc)?,
            OpCode::DefineGlobal => self.define_global_op(frame, loc)?,
            OpCode::GetGlobal => self.get_global_op(frame, loc)?,
            OpCode::SetGlobal => self.set_global_op(frame, loc)?,
            OpCode::GetLocal(slot) => self.get_local_op(frame, slot, loc)?,
            OpCode::SetLocal(slot) => self.set_local_op(frame, slot, loc)?,
            OpCode::JumpIfFalse(jump_offset) => self.jump_if_false_op(frame, jump_offset, loc)?,
            OpCode::Jump(jump_offset) => self.jump_op(frame, jump_offset)?,
            OpCode::Loop(loop_offset) => self.loop_op(frame, loop_offset)?,
        }

        Ok(())
    }

    fn return_op(&self, _loc: &Location) -> Result<(), InterpretError> {
        Ok(())
    }

    fn negate_op(&self, frame: &mut CallFrame, loc: &Location) -> Result<(), InterpretError> {
        match frame.slots.pop() {
            Some(Value::Number(value)) => {
                self.stack_push(frame, Value::Number(-value));
                Ok(())
            }
            _ => self.report_error(loc, "Operand is not a number value"),
        }
    }

    fn not_op(&self, frame: &mut CallFrame, loc: &Location) -> Result<(), InterpretError> {
        match frame.slots.pop() {
            Some(value) => {
                self.stack_push(frame, Value::Boolean(value.is_falsy()));
                Ok(())
            }
            _ => self.report_error(loc, "Operand must have a value"),
        }
    }

    fn binary_op(
        &self,
        frame: &mut CallFrame,
        op: fn(Value, Value) -> Value,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match (frame.slots.pop(), frame.slots.pop()) {
            (Some(a), Some(b)) => {
                self.stack_push(frame, op(b, a));
                Ok(())
            }
            _ => self.report_error(loc, "Operands must have two values"),
        }
    }

    fn print_op(&self, frame: &mut CallFrame, loc: &Location) -> Result<(), InterpretError> {
        if let Some(value) = frame.slots.pop() {
            println!("{}", value);
            Ok(())
        } else {
            self.report_error(loc, "No value to print")
        }
    }

    fn pop_op(&mut self, frame: &mut CallFrame, loc: &Location) -> Result<(), InterpretError> {
        if frame.slots.is_empty() {
            self.report_error(loc, "No value to pop")
        } else {
            frame.slots.pop();
            Ok(())
        }
    }

    fn define_global_op(
        &mut self,
        frame: &mut CallFrame,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match (frame.slots.pop(), frame.slots.pop()) {
            (Some(value), Some(Value::Object(Object::String(key)))) => {
                self.globals.set(&key, value);
                Ok(())
            }
            _ => self.report_error(loc, "No valid values to define"),
        }
    }

    fn get_global_op(
        &mut self,
        frame: &mut CallFrame,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match frame.slots.pop() {
            Some(Value::Object(Object::String(key))) => {
                if let Some(value) = self.globals.get(&key) {
                    self.stack_push(frame, value.clone());
                    Ok(())
                } else {
                    self.report_error(loc, "Undefined global variable")
                }
            }
            _ => self.report_error(loc, "No valid values to get"),
        }
    }

    fn set_global_op(
        &mut self,
        frame: &mut CallFrame,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match (frame.slots.pop(), frame.slots.last()) {
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

    fn get_local_op(
        &mut self,
        frame: &mut CallFrame,
        slot: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = frame.slots.get(slot) {
            self.stack_push(frame, value.clone());
            Ok(())
        } else {
            self.report_error(loc, "Undefined local variable")
        }
    }

    fn set_local_op(
        &mut self,
        frame: &mut CallFrame,
        slot: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        match (frame.slots.last(), frame.slots.get(slot)) {
            (Some(value), Some(_)) => {
                frame.slots[slot] = value.clone();
                Ok(())
            }
            _ => self.report_error(loc, "No valid values to set"),
        }
    }

    fn jump_if_false_op(
        &mut self,
        frame: &mut CallFrame,
        jump_offset: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = frame.slots.last() {
            if value.is_falsy() {
                frame.ip += jump_offset;
            }
            Ok(())
        } else {
            self.report_error(loc, "No value to if else condition")
        }
    }

    fn jump_op(&mut self, frame: &mut CallFrame, jump_offset: usize) -> Result<(), InterpretError> {
        frame.ip += jump_offset;
        Ok(())
    }

    fn loop_op(&mut self, frame: &mut CallFrame, loop_offset: usize) -> Result<(), InterpretError> {
        frame.ip -= loop_offset;
        Ok(())
    }

    fn stack_push(&self, frame: &mut CallFrame, value: Value) {
        // if let Value::Object(object) = value.clone() {
        //     self.object_list = Some(Rc::new(RefCell::new(ObjectNode {
        //         value: object,
        //         next: self.object_list.clone(),
        //     })));
        // }
        frame.slots.push(value);
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
