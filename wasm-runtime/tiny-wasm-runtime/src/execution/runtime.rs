use super::{
    import::Import,
    store::{ExternalFuncInst, FuncInst, InternalFuncInst, Store},
    value::Value,
};
use crate::binary::{
    instruction::Instruction,
    module::Module,
    types::{ExportDesc, ValueType},
};
use anyhow::{anyhow, bail, Result};

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
    pub import: Import,
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
                Instruction::Call(idx) => {
                    let Some(func) = self.store.funcs.get(*idx as usize) else {
                        bail!("not found function.")
                    };
                    let func_inst = func.clone();
                    match func_inst {
                        FuncInst::Internal(func) => self.push_frame(&func),
                        FuncInst::External(func) => {
                            // 外部関数の呼び出しは call stack に積まない、結果だけを stack に積む
                            if let Some(value) = self.invoke_external(func)? {
                                self.stack.push(value);
                            }
                        }
                    }
                }
            };
        }
        Ok(())
    }

    pub fn call(&mut self, name: impl Into<String>, args: Vec<Value>) -> Result<Option<Value>> {
        let idx = match self
            .store
            .module
            .exports
            .get(&name.into())
            .ok_or(anyhow!("not found export function"))?
            .desc
        {
            ExportDesc::Func(idx) => idx as usize,
        };
        let Some(func) = self.store.funcs.get(idx) else {
            bail!("not found function.")
        };
        // 引数はあらかじめ stack に積んでおく
        for arg in args {
            self.stack.push(arg);
        }
        match func {
            FuncInst::Internal(func) => self.invoke_internal(func.clone()),
            FuncInst::External(func) => self.invoke_external(func.clone()),
        }
    }

    pub fn add_import(
        &mut self,
        module_name: impl Into<String>,
        func_name: impl Into<String>,
        func: impl FnMut(&mut Store, Vec<Value>) -> Result<Option<Value>> + 'static,
    ) -> Result<()> {
        let import = self.import.entry(module_name.into()).or_default();
        import.insert(func_name.into(), Box::new(func));
        Ok(())
    }

    fn push_frame(&mut self, func: &InternalFuncInst) {
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
    }

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        // 戻り値の数を取得
        let arity = func.func_type.results.len();
        // Frame を作成
        self.push_frame(&func);

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

    fn invoke_external(&mut self, func: ExternalFuncInst) -> Result<Option<Value>> {
        let bottom = self.stack.len() - func.func_type.params.len();
        let args = self.stack.split_off(bottom);
        let module = self
            .import
            .get_mut(&func.module)
            .ok_or(anyhow!("not found module."))?;
        let import_func = module
            .get_mut(&func.func)
            .ok_or(anyhow!("not found function."))?;
        import_func(&mut self.store, args)
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
            let result = runtime.call("add", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }

        Ok(())
    }

    #[test]
    fn not_found_export_function() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let result = runtime.call("foooooo", vec![]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn func_call() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_call.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let tests = vec![(2, 4), (10, 20), (1, 2)];
        for (arg, want) in tests {
            let args = vec![Value::I32(arg)];
            let result = runtime.call("call_doubler", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }

    #[test]
    fn not_found_import_function() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        runtime.add_import("env", "foooo", |_, _| Ok(None))?;
        let result = runtime.call("call_add", vec![Value::I32(1)]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn call_imported_func() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        runtime.add_import("env", "add", |_, args| {
            let arg = args[0];
            Ok(Some(arg + arg))
        })?;
        let tests = vec![(2, 4), (10, 20), (1, 2)];
        for (arg, want) in tests {
            let args = vec![Value::I32(arg)];
            let result = runtime.call("call_add", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }
}
