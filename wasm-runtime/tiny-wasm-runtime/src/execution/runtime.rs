use std::mem::size_of;

use super::{
    import::Import,
    store::{ExternalFuncInst, FuncInst, InternalFuncInst, Store},
    value::{Label, LabelKind, Value},
    wasi::WasiSnapShotPreview1,
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
    pub labels: Vec<Label>,
}

#[derive(Default)]
pub struct Runtime {
    pub store: Store,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
    pub import: Import,
    pub wasi: Option<WasiSnapShotPreview1>,
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

    pub fn instantiate_with_wasi(
        wasm: impl AsRef<[u8]>,
        wasi: WasiSnapShotPreview1,
    ) -> Result<Self> {
        let runtime = Module::new(wasm.as_ref())?;
        let module = Store::new(runtime)?;
        Ok(Self {
            store: module,
            wasi: Some(wasi),
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
                Instruction::If(block) => {
                    let cond = self.stack.pop().ok_or(anyhow!("not found condition."))?;

                    // if の終わりの program counter を取得
                    let next_pc = get_end_address(&frame.insts, frame.pc as usize)?;
                    // cond が 0 のとき = false のときなので、pc を next_pc に移動させる
                    if cond == Value::I32(0) {
                        frame.pc = next_pc as isize;
                    }

                    let label = Label {
                        kind: LabelKind::If,
                        pc: next_pc,
                        sp: self.stack.len(),
                        arity: block.block_type.result_count(),
                    };
                    frame.labels.push(label);
                }
                Instruction::End => match frame.labels.pop() {
                    // if / block / loop の終わり
                    Some(label) => {
                        let Label { pc, sp, arity, .. } = label;
                        // program counter を移動させて、スタックを戻す
                        // 関数呼び出しではないので、call stack は変更しない
                        frame.pc = pc as isize;
                        stack_unwind(&mut self.stack, sp, arity)?;
                    }
                    // 関数の終わり
                    None => {
                        let Some(frame) = self.call_stack.pop() else {
                            bail!("not found frame.")
                        };
                        let Frame { sp, arity, .. } = frame;
                        stack_unwind(&mut self.stack, sp, arity)?;
                    }
                },
                Instruction::Return => {
                    // return は必ず関数の終わりなので、call stack から frame を取り出す
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
                Instruction::LocalSet(idx) => {
                    let Some(value) = self.stack.pop() else {
                        bail!("not found value in the stack.")
                    };
                    frame.locals[*idx as usize] = value;
                }
                Instruction::I32Store { align: _, offset } => {
                    // メモリに書き込む値とアドレスを取得
                    // → i32.store を呼び出す前には i32.const などでスタックにこれらの値を積んでおく必要がある
                    let (Some(value), Some(addr)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack.")
                    };
                    // 値が i32 であることをベースに書き込む範囲 (at と end) を計算する
                    let addr = Into::<i32>::into(addr) as usize;
                    let offset = (*offset) as usize;
                    let at = addr + offset;
                    let end = at + size_of::<i32>();
                    let memory = self
                        .store
                        .memories
                        .get_mut(0)
                        .ok_or(anyhow!("not found memory."))?;
                    let value: i32 = value.into();
                    memory.data[at..end].copy_from_slice(&value.to_le_bytes());
                }
                Instruction::I32Const(value) => {
                    self.stack.push(Value::I32(*value));
                }
                Instruction::I32Add => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack.")
                    };
                    let result = left + right;
                    self.stack.push(result);
                }
                Instruction::I32Sub => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack.")
                    };
                    let result = left - right;
                    self.stack.push(result);
                }
                Instruction::I32Lts => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack.")
                    };
                    let result = left < right;
                    self.stack.push(Value::I32(result.into()));
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
            labels: vec![],
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

        if func.module == "wasi_snapshot_preview1" {
            if let Some(wasi) = &mut self.wasi {
                return wasi.invoke(&mut self.store, &func.func, args);
            }
        }

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

pub fn get_end_address(insts: &[Instruction], pc: usize) -> Result<usize> {
    let mut pc = pc;
    let mut depth = 0;
    loop {
        pc += 1;
        let inst = insts.get(pc).ok_or(anyhow!("not found instruction."))?;
        match inst {
            // if がネストしている場合があるので、depth を使って終了を判断する
            Instruction::If(_) => depth += 1,
            Instruction::End => {
                if depth == 0 {
                    return Ok(pc);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
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

    #[test]
    fn i32_const() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/i32_const.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let result = runtime.call("i32_const", vec![])?;
        assert_eq!(result, Some(Value::I32(42)));
        Ok(())
    }

    #[test]
    fn local_set() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/local_set.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let result = runtime.call("local_set", vec![])?;
        assert_eq!(result, Some(Value::I32(42)));
        Ok(())
    }

    #[test]
    fn i32_store() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/i32_store.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        runtime.call("i32_store", vec![])?;
        let memory = &runtime.store.memories[0].data;
        assert_eq!(memory[0], 42);
        Ok(())
    }

    #[test]
    fn i32_sub() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_sub.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let result = runtime.call("sub", vec![Value::I32(10), Value::I32(5)])?;
        assert_eq!(result, Some(Value::I32(5)));
        Ok(())
    }

    #[test]
    fn i32_lts() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_lts.wat")?;
        let mut runtime = Runtime::instantiate(&wasm)?;
        let result = runtime.call("lts", vec![Value::I32(10), Value::I32(5)])?;
        assert_eq!(result, Some(Value::I32(0)));
        Ok(())
    }

    #[test]
    fn fib() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/fib.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        let tests = vec![
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 5),
            (5, 8),
            (6, 13),
            (7, 21),
            (8, 34),
            (9, 55),
            (10, 89),
        ];

        for (arg, want) in tests {
            let args = vec![Value::I32(arg)];
            let result = runtime.call("fib", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }
}
