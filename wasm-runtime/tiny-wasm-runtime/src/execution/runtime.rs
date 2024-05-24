use super::{
    store::{FuncInst, InternalFuncInst, Store},
    value::Value,
};
use crate::binary::{instruction::Instruction, module::Module, types::ValueType};
use anyhow::{bail, Result};

#[derive(Default)]
pub struct Frame {
    pub pc: isize, // プログラムカウンタ
    pub sp: usize, // スタックポインタ
    pub insts: Vec<Instruction>,
    pub arity: usize,       // 戻り値の数
    pub locals: Vec<Value>, // ローカル変数
}

#[derive(Default)]
pub struct Runtime {
    pub store: Store,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
}

impl Runtime {
    pub fn instantiate(wasm: impl AsRef<[u8]>) -> Result<Self> {
        let module = Module::new(wasm.as_ref())?;
        let store = Store::new(module)?;
        Ok(Self {
            store,
            ..Default::default()
        })
    }

    fn execute(&mut self) -> Result<()> {
        loop {
            let Some(frame) = self.call_stack.last_mut() else {
                break;
            };
            frame.pc += 1;
            // ここ 1 から始まるけど、0 から始まるべきでは？
            // → invoke_internal で pc = -1 から始めているので問題なかった
            let Some(inst) = frame.insts.get(frame.pc as usize) else {
                break;
            };
            match inst {
                Instruction::End => {
                    let Some(frame) = self.call_stack.pop() else {
                        bail!("not found frame.")
                    };
                    let Frame { sp, arity, .. } = frame;
                    stack_unwind(&mut self.stack, sp, arity)?;
                }
                Instruction::LocalGet(idx) => {
                    let Some(local_value) = frame.locals.get(*idx as usize) else {
                        bail!("not found local variable.")
                    };
                    self.stack.push(*local_value);
                }
                Instruction::I32Add => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack.")
                    };
                    let result = left + right;
                    self.stack.push(result);
                }
            };
        }
        Ok(())
    }

    pub fn call(&mut self, idx: usize, args: Vec<Value>) -> Result<Option<Value>> {
        let Some(func) = self.store.funcs.get(idx) else {
            bail!("not found function.")
        };
        // 引数はあらかじめ stack に積んでおく
        for arg in args {
            self.stack.push(arg);
        }
        match func {
            FuncInst::Internal(func) => self.invoke_internal(func.clone()),
        }
    }

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        let bottom = self.stack.len() - func.func_type.params.len();
        // 引数を残して stack を取り出す (直前に積まれていると仮定して良い？)
        // → call メソッド見ると、invoke_internal が呼ばれる前に引数を積んでいるので問題ない
        let mut locals = self.stack.split_off(bottom);

        // ローカル変数を stack に積む
        for local in func.code.locals.iter() {
            match local {
                ValueType::I32 => locals.push(Value::I32(0)),
                ValueType::I64 => locals.push(Value::I64(0)),
            }
        }

        // 戻り値の数を取得
        let arity = func.func_type.results.len();
        // Frame を作成
        let frame = Frame {
            pc: -1,
            // 現在の stack の長さを保存 (後で stack を戻すため)
            sp: self.stack.len(),
            insts: func.code.body.clone(),
            arity,
            locals,
        };

        self.call_stack.push(frame);

        if let Err(e) = self.execute() {
            self.cleanup();
            bail!("failed to execute instructions: {:?}", e);
        };

        if arity > 0 {
            let Some(value) = self.stack.pop() else {
                bail!("not found return value.")
            };
            return Ok(Some(value));
        }
        Ok(None)
    }

    fn cleanup(&mut self) {
        self.stack = vec![];
        self.call_stack = vec![];
    }
}

pub fn stack_unwind(stack: &mut Vec<Value>, sp: usize, arity: usize) -> Result<()> {
    if arity > 0 {
        let Some(value) = stack.pop() else {
            bail!("not found return value.")
        };
        stack.drain(sp..);
        stack.push(value);
    } else {
        stack.drain(sp..);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::execution::value::Value;

    use super::Runtime;
    use anyhow::Result;

    #[test]
    fn execute_i32_add() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let tests = vec![(2, 3, 5), (10, 5, 15), (1, 1, 2)];

        for (left, right, want) in tests {
            let args = vec![Value::I32(left), Value::I32(right)];
            let result = runtime.call(0, args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }

        Ok(())
    }
}
