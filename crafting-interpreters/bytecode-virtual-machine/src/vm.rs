use crate::{
    code::OpCode,
    compiler::{CompileError, Compiler, UpValue},
    table::Table,
    token::Location,
    value::{Closure, Object, Value},
};

#[derive(Debug)]
pub enum InterpretError {
    RuntimeError,
    CompileError(()),
}

impl From<CompileError> for InterpretError {
    fn from(_: CompileError) -> Self {
        InterpretError::CompileError(())
    }
}

#[derive(Debug, Clone)]
struct CallFrame {
    closure: Closure,
    ip: usize,
    stack_top: usize,
}

impl CallFrame {
    fn new(closure: Closure, stack_top: usize) -> Self {
        CallFrame {
            closure,
            ip: 0,
            stack_top,
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
    globals: Table,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    frame_cnt: usize,
    // pub object_list: Option<Rc<RefCell<ObjectNode>>>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            globals: Table::new(30),
            frames: Vec::new(),
            stack: Vec::new(),
            frame_cnt: 0,
            // object_list: None,
        }
    }

    pub fn interpret(&mut self, source: String) -> Result<(), InterpretError> {
        let mut compiler = Compiler::new(source);
        let function = compiler.compile()?;
        self.frames.push(CallFrame::new(Closure::new(function), 0));
        self.frame_cnt += 1;
        self.run()
    }

    fn run(&mut self) -> Result<(), InterpretError> {
        if let Some(mut frame) = self.frames.pop() {
            tracing::debug!("Running frame: {:?}", frame);
            tracing::debug!("=============================");
            while frame.ip < frame.closure.function.codes.len() {
                let (instruction, loc) = frame.closure.function.codes[frame.ip].clone();
                tracing::debug!("Running instruction: {:?} at {:?}", instruction, loc);
                tracing::debug!("Before Stack: {:?}", self.stack);
                // tracing::debug!("Closure: {:?}", frame.closure);
                self.execute_operation(instruction, &mut frame, &loc)?;
                frame.ip += 1;
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
            OpCode::Return => self.return_op(frame.stack_top, loc)?,
            OpCode::Constant(value) => self.stack_push(value.clone()),
            OpCode::Nil => self.stack_push(Value::Nil),
            OpCode::True => self.stack_push(Value::Boolean(true)),
            OpCode::False => self.stack_push(Value::Boolean(false)),
            OpCode::Negate => self.negate_op(loc)?,
            OpCode::Not => self.not_op(loc)?,
            OpCode::Add => self.binary_op(|a, b| a + b, loc)?,
            OpCode::Subtract => self.binary_op(|a, b| a - b, loc)?,
            OpCode::Multiply => self.binary_op(|a, b| a * b, loc)?,
            OpCode::Divide => self.binary_op(|a, b| a / b, loc)?,
            OpCode::Equal => self.binary_op(|a, b| Value::Boolean(a == b), loc)?,
            OpCode::Greater => self.binary_op(|a, b| Value::Boolean(a > b), loc)?,
            OpCode::Less => self.binary_op(|a, b| Value::Boolean(a < b), loc)?,
            OpCode::Print => self.print_op(loc)?,
            OpCode::Pop => self.pop_op(loc)?,
            OpCode::DefineGlobal => self.define_global_op(loc)?,
            OpCode::GetGlobal => self.get_global_op(loc)?,
            OpCode::SetGlobal => self.set_global_op(loc)?,
            OpCode::GetLocal(slot) => self.get_local_op(slot + frame.stack_top, loc)?,
            OpCode::SetLocal(slot) => self.set_local_op(slot + frame.stack_top, loc)?,
            OpCode::JumpIfFalse(jump_offset) => self.jump_if_false_op(frame, jump_offset, loc)?,
            OpCode::Jump(jump_offset) => self.jump_op(frame, jump_offset)?,
            OpCode::Loop(loop_offset) => self.loop_op(frame, loop_offset)?,
            OpCode::Call(arg_count) => self.call_op(arg_count, loc)?,
            OpCode::Closure(object, up_values) => self.closure_op(frame, object, up_values, loc)?,
            OpCode::GetUpValue(slot) => self.get_up_value_op(frame, slot, loc)?,
            OpCode::SetUpValue(slot) => self.set_up_value_op(frame, slot, loc)?,
        }

        Ok(())
    }

    fn return_op(&mut self, stack_top: usize, loc: &Location) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.pop() {
            self.frames.pop();
            self.frame_cnt -= 1;
            if self.frame_cnt == 0 {
                self.stack.pop();
                return Ok(());
            }
            self.stack.drain(stack_top - 1..);
            self.stack_push(value);
            Ok(())
        } else {
            self.report_error(loc, "No value to return")
        }
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
        frame: &mut CallFrame,
        jump_offset: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.last() {
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

    fn call_op(&mut self, arg_count: usize, loc: &Location) -> Result<(), InterpretError> {
        let stack_top = self.stack.len() - arg_count - 1;
        if let Value::Object(Object::Closure(closure)) = self.stack.get(stack_top).unwrap() {
            if closure.function.arity != arg_count {
                return self.report_error(loc, "Incorrect number of arguments");
            }
            self.frames
                .push(CallFrame::new(closure.clone(), stack_top + 1));
            self.frame_cnt += 1;
            self.run()?;
            Ok(())
        } else {
            self.report_error(loc, "Can only call functions and closures")
        }
    }

    fn closure_op(
        &mut self,
        frame: &CallFrame,
        object: Object,
        up_values: Vec<UpValue>,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Object::Function(function) = object {
            let mut closure = Closure::new(function);
            for up_value in up_values {
                if up_value.is_local {
                    closure
                        .up_values
                        .push(self.stack[frame.stack_top + up_value.index].clone());
                } else {
                    closure
                        .up_values
                        .push(frame.closure.up_values[up_value.index].clone());
                }
            }
            self.stack_push(Value::Object(Object::Closure(closure)));
            Ok(())
        } else {
            self.report_error(loc, "Can only create closures from functions")
        }
    }

    fn get_up_value_op(
        &mut self,
        frame: &CallFrame,
        slot: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = frame.closure.up_values.get(slot) {
            self.stack_push(value.clone());
            Ok(())
        } else {
            self.report_error(loc, "No up value to get")
        }
    }

    fn set_up_value_op(
        &mut self,
        frame: &mut CallFrame,
        slot: usize,
        loc: &Location,
    ) -> Result<(), InterpretError> {
        if let Some(value) = self.stack.last() {
            frame.closure.up_values[slot] = value.clone();
            Ok(())
        } else {
            self.report_error(loc, "No up value to set")
        }
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
