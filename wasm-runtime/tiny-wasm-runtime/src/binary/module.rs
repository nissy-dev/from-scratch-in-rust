use anyhow::Result;
use nom::{
    bytes::complete::{tag, take},
    multi::many0,
    number::complete::{le_u32, le_u8},
    sequence::pair,
    IResult,
};
use nom_leb128::{leb128_i32, leb128_u32};
use num_traits::FromPrimitive as _;

use super::{
    instruction::Instruction,
    opcode::Opcode,
    section::{Function, SectionCode},
    types::{
        Block, BlockType, Data, Export, ExportDesc, FuncType, FunctionLocal, Import, ImportDesc,
        Limits, Memory, ValueType,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    // メモリの設定を保持する
    pub memory_section: Option<Vec<Memory>>,
    // メモリの初期化データを保持する
    pub data_section: Option<Vec<Data>>,
    // 関数の型を保持する
    pub type_section: Option<Vec<FuncType>>,
    // 関数の型と実装の対応を保持する
    pub function_section: Option<Vec<u32>>,
    // 関数の実装を保持する
    pub code_section: Option<Vec<Function>>,
    // エクスポートされた値を保持する
    pub export_section: Option<Vec<Export>>,
    // インポートされた値を保持する
    pub import_section: Option<Vec<Import>>,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            magic: "\0asm".to_string(),
            version: 1,
            memory_section: None,
            data_section: None,
            type_section: None,
            function_section: None,
            code_section: None,
            export_section: None,
            import_section: None,
        }
    }
}

impl Module {
    pub fn new(input: &[u8]) -> Result<Module> {
        let (_, module) =
            Module::decode(input).map_err(|e| anyhow::anyhow!("failed to parse wasm: {}", e))?;
        Ok(module)
    }

    fn decode(input: &[u8]) -> IResult<&[u8], Module> {
        let (input, _) = tag(b"\0asm")(input)?;
        let (input, version) = le_u32(input)?;
        let mut module = Module {
            magic: "\0asm".to_string(),
            version,
            ..Default::default()
        };
        let mut remaining = input;
        while !remaining.is_empty() {
            match decode_section_header(remaining) {
                Ok((input, (code, size))) => {
                    let (rest, section_contents) = take(size)(input)?;
                    match code {
                        SectionCode::Custom => {
                            // カスタムセクションは無視する
                        }
                        SectionCode::Memory => {
                            let (_, memory) = decode_memory_section(section_contents)?;
                            module.memory_section = Some(vec![memory]);
                        }
                        SectionCode::Data => {
                            let (_, data) = decode_data_section(section_contents)?;
                            module.data_section = Some(data);
                        }
                        SectionCode::Type => {
                            let (_, types) = decode_type_section(section_contents)?;
                            module.type_section = Some(types);
                        }
                        SectionCode::Function => {
                            let (_, func_idx_list) = decode_function_section(section_contents)?;
                            module.function_section = Some(func_idx_list);
                        }
                        SectionCode::Code => {
                            let (_, functions) = decode_code_section(section_contents)?;
                            module.code_section = Some(functions);
                        }
                        SectionCode::Export => {
                            let (_, exports) = decode_export_section(section_contents)?;
                            module.export_section = Some(exports);
                        }
                        SectionCode::Import => {
                            let (_, imports) = decode_import_section(section_contents)?;
                            module.import_section = Some(imports);
                        }
                    };
                    remaining = rest;
                }
                Err(err) => return Err(err),
            }
        }
        Ok((input, module))
    }
}

fn decode_section_header(input: &[u8]) -> IResult<&[u8], (SectionCode, u32)> {
    let (input, (code, size)) = pair(le_u8, leb128_u32)(input)?;
    Ok((
        input,
        (
            SectionCode::from_u8(code).expect("unexpected section code"),
            size,
        ),
    ))
}

fn decode_type_section(input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
    let mut func_types = vec![];
    // : num types
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        // 今回は関数型しか扱わないので、読み取っても値を使わない
        // ; func
        let (rest, _) = le_u8(input)?;
        let mut func = FuncType::default();

        // ; num params
        let (rest, num_params) = leb128_u32(rest)?;
        let (rest, params) = take(num_params)(rest)?;
        let (_, params) = many0(decode_value_type)(params)?;
        func.params = params;

        // ; num results
        let (rest, num_results) = leb128_u32(rest)?;
        let (rest, results) = take(num_results)(rest)?;
        let (_, results) = many0(decode_value_type)(results)?;
        func.results = results;

        func_types.push(func);
        input = rest;
    }

    Ok((&[], func_types))
}

fn decode_value_type(input: &[u8]) -> IResult<&[u8], ValueType> {
    let (input, value_type) = le_u8(input)?;
    Ok((input, value_type.into()))
}

fn decode_function_section(input: &[u8]) -> IResult<&[u8], Vec<u32>> {
    let mut func_idx_list = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, idx) = leb128_u32(input)?;
        func_idx_list.push(idx);
        input = rest
    }

    Ok((&[], func_idx_list))
}

fn decode_code_section(input: &[u8]) -> IResult<&[u8], Vec<Function>> {
    let mut functions = vec![];
    // : num functions
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        // ; func body size
        let (rest, body_size) = leb128_u32(input)?;
        let (rest, body) = take(body_size)(rest)?;
        let (_, body) = decode_function_body(body)?;
        functions.push(body);
        input = rest;
    }

    Ok((&[], functions))
}

fn decode_function_body(input: &[u8]) -> IResult<&[u8], Function> {
    let mut body = Function::default();
    // ; local decl count
    let (mut input, local_decl_count) = leb128_u32(input)?;
    for _ in 0..local_decl_count {
        // ; local type count
        let (rest, local_type_count) = leb128_u32(input)?;
        // ; i32
        let (rest, value_type) = le_u8(rest)?;
        body.locals.push(FunctionLocal {
            type_count: local_type_count,
            value_type: value_type.into(),
        });
        input = rest;
    }

    let (_, instructions) = many0(decode_instructions)(input)?;
    body.code = instructions;

    Ok((&[], body))
}

fn decode_instructions(input: &[u8]) -> IResult<&[u8], Instruction> {
    let (input, byte) = le_u8(input)?;
    let opcode = Opcode::from_u8(byte).unwrap_or_else(|| panic!("invalid opcode: {:X}", byte));
    let (rest, instruction) = match opcode {
        Opcode::If => {
            let (rest, block) = decode_block(input)?;
            (rest, Instruction::If(block))
        }
        Opcode::Return => (input, Instruction::Return),
        Opcode::LocalGet => {
            let (rest, local_idx) = leb128_u32(input)?;
            (rest, Instruction::LocalGet(local_idx))
        }
        Opcode::LocalSet => {
            let (rest, local_idx) = leb128_u32(input)?;
            (rest, Instruction::LocalSet(local_idx))
        }
        Opcode::I32Store => {
            // アライメントはメモリの境界値チェックのために使われるが、この本では扱わない
            let (rest, align) = leb128_u32(input)?;
            // アドレス + オフセットの箇所に実際に値を書き込む
            let (rest, offset) = leb128_u32(rest)?;
            (rest, Instruction::I32Store { align, offset })
        }
        Opcode::I32Const => {
            let (rest, value) = leb128_i32(input)?;
            (rest, Instruction::I32Const(value))
        }
        Opcode::I32Lts => (input, Instruction::I32Lts),
        Opcode::I32Add => (input, Instruction::I32Add),
        Opcode::I32Sub => (input, Instruction::I32Sub),
        Opcode::End => (input, Instruction::End),
        Opcode::Call => {
            let (rest, func_idx) = leb128_u32(input)?;
            (rest, Instruction::Call(func_idx))
        }
    };
    Ok((rest, instruction))
}

fn decode_export_section(input: &[u8]) -> IResult<&[u8], Vec<Export>> {
    let mut exports = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, name) = decode_name(input)?;
        let (rest, export_kind) = le_u8(rest)?;
        let (rest, desc) = match export_kind {
            0x00 => {
                let (rest, idx) = leb128_u32(rest)?;
                (rest, ExportDesc::Func(idx))
            }
            _ => unimplemented!("unsupported export kind: {:X}", export_kind),
        };
        exports.push(Export { name, desc });
        input = rest;
    }

    Ok((&[], exports))
}

fn decode_import_section(input: &[u8]) -> IResult<&[u8], Vec<Import>> {
    let mut imports = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, module) = decode_name(input)?;
        let (rest, field) = decode_name(rest)?;
        let (rest, import_kind) = le_u8(rest)?;
        let (rest, desc) = match import_kind {
            0x00 => {
                let (rest, idx) = leb128_u32(rest)?;
                (rest, ImportDesc::Func(idx))
            }
            _ => unimplemented!("unsupported import kind: {:X}", import_kind),
        };
        imports.push(Import {
            module,
            field,
            desc,
        });
        input = rest;
    }

    Ok((&[], imports))
}

fn decode_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, name_len) = leb128_u32(input)?;
    let (input, name_bytes) = take(name_len)(input)?;
    let name = String::from_utf8(name_bytes.to_vec()).expect("invalid utf8 string");
    Ok((input, name))
}

fn decode_memory_section(input: &[u8]) -> IResult<&[u8], Memory> {
    // メモリの数を読み取るのだが、version 1 では常に 1 なので読み取っても使わない
    let (input, _) = leb128_u32(input)?;
    let (_, limits) = decode_limits(input)?;
    Ok((input, Memory { limits }))
}

fn decode_limits(input: &[u8]) -> IResult<&[u8], Limits> {
    let (input, (flags, min)) = pair(leb128_u32, leb128_u32)(input)?;
    let max = if flags == 0 {
        None
    } else {
        let (_, max) = leb128_u32(input)?;
        Some(max)
    };
    Ok((input, Limits { min, max }))
}

// offset を指定する命令列をデコードする
// 今回は [i32.const, offset, end] の命令列になっていることを前提とする
fn decode_expr(input: &[u8]) -> IResult<&[u8], u32> {
    // i32.const 命令を読み取る
    let (input, _) = leb128_u32(input)?;
    // 実際の offset の数値が格納されているバイト列を読み取る
    let (input, offset) = leb128_u32(input)?;
    // end 命令を読み取る
    let (input, _) = leb128_u32(input)?;
    Ok((input, offset))
}

fn decode_data_section(input: &[u8]) -> IResult<&[u8], Vec<Data>> {
    let (mut input, count) = leb128_u32(input)?;
    let mut data = vec![];
    for _ in 0..count {
        let (rest, memory_idx) = leb128_u32(input)?;
        let (rest, offset) = decode_expr(rest)?;
        let (rest, size) = leb128_u32(rest)?;
        let (rest, init) = take(size)(rest)?;
        data.push(Data {
            memory_idx,
            offset,
            init: init.to_vec(),
        });
        input = rest;
    }
    Ok((input, data))
}

fn decode_block(input: &[u8]) -> IResult<&[u8], Block> {
    let (input, byte) = le_u8(input)?;
    let block_type = match byte {
        0x40 => BlockType::Void,
        _ => BlockType::ValueType(vec![byte.into()]),
    };
    Ok((input, Block { block_type }))
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use anyhow::{Ok, Result};

    #[test]
    fn decode_simplest_module() -> Result<()> {
        // プリアンブルしか存在しないwasmバイナリ
        // プリアンブル: コンパイルに先立って予備的な指示を与えるプリプロセッサーを記述した部分
        let wasm = wat::parse_str(r#"(module)"#)?;
        // バイナリをデコードしてModule構造体を作成
        let module = Module::new(&wasm)?;
        // 生成したModule構造体が想定通りになっているか確認
        assert_eq!(module, Module::default());
        Ok(())
    }

    #[test]
    fn decode_simplest_func() -> Result<()> {
        let wasm = wat::parse_str(
            r#"
        (module
          (func)
        )"#,
        )?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType::default()]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_param() -> Result<()> {
        let wasm = wat::parse_str(
            r#"
        (module
          (func (param i32 i64))
        )
        "#,
        )?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I64],
                    results: vec![],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_local() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_local.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType::default()]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![
                        FunctionLocal {
                            type_count: 1,
                            value_type: ValueType::I32,
                        },
                        FunctionLocal {
                            type_count: 2,
                            value_type: ValueType::I64,
                        },
                    ],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_add() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::LocalGet(1),
                        Instruction::I32Add,
                        Instruction::End,
                    ],
                }]),
                export_section: Some(vec![Export {
                    name: "add".to_string(),
                    desc: ExportDesc::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_call() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_call.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0, 0]),
                code_section: Some(vec![
                    Function {
                        locals: vec![],
                        code: vec![
                            Instruction::LocalGet(0),
                            Instruction::Call(1),
                            Instruction::End,
                        ],
                    },
                    Function {
                        locals: vec![],
                        code: vec![
                            Instruction::LocalGet(0),
                            Instruction::LocalGet(0),
                            Instruction::I32Add,
                            Instruction::End,
                        ],
                    }
                ]),
                export_section: Some(vec![Export {
                    name: "call_doubler".to_string(),
                    desc: ExportDesc::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_import() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::Call(0),
                        Instruction::End,
                    ],
                }]),
                import_section: Some(vec![Import {
                    module: "env".to_string(),
                    field: "add".to_string(),
                    desc: ImportDesc::Func(0),
                }]),
                export_section: Some(vec![Export {
                    name: "call_add".to_string(),
                    desc: ExportDesc::Func(1),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_i32_store() -> Result<()> {
        let wasm = wat::parse_str("(module (func (i32.store offset=4 (i32.const 4))))")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::I32Const(4),
                        Instruction::I32Store {
                            align: 2,
                            offset: 4
                        },
                        Instruction::End,
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_memory() -> Result<()> {
        let tests = vec![
            ("(module (memory 1))", Limits { min: 1, max: None }),
            (
                "(module (memory 1 2))",
                Limits {
                    min: 1,
                    max: Some(2),
                },
            ),
        ];

        for (wasm, limits) in tests {
            let module = Module::new(&wat::parse_str(wasm)?)?;
            assert_eq!(
                module,
                Module {
                    memory_section: Some(vec![Memory { limits }]),
                    ..Default::default()
                }
            );
        }
        Ok(())
    }

    #[test]
    fn decode_data() -> Result<()> {
        let tests =
            vec![
            (
                "(module (memory 1) (data (i32.const 0) \"hello\"))",
                vec![Data {
                    memory_idx: 0,
                    offset: 0,
                    init: "hello".as_bytes().to_vec(),
                }],
            ),
            (
                "(module (memory 1) (data (i32.const 0) \"hello\") (data (i32.const 5) \"world\"))",
                vec![Data {
                    memory_idx: 0,
                    offset: 0,
                    init: b"hello".to_vec(),
                },
                Data {
                    memory_idx: 0,
                    offset: 5,
                    init: b"world".to_vec(),
                }],
            ),
        ];

        for (wasm, data) in tests {
            let module = Module::new(&wat::parse_str(wasm)?)?;
            assert_eq!(
                module,
                Module {
                    memory_section: Some(vec![Memory {
                        limits: Limits { min: 1, max: None }
                    }]),
                    data_section: Some(data),
                    ..Default::default()
                }
            );
        }
        Ok(())
    }

    #[test]
    fn decode_fib() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/fib.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::I32Const(2),
                        Instruction::I32Lts,
                        Instruction::If(Block {
                            block_type: BlockType::Void
                        }),
                        Instruction::I32Const(1),
                        Instruction::Return,
                        Instruction::End,
                        Instruction::LocalGet(0),
                        Instruction::I32Const(2),
                        Instruction::I32Sub,
                        Instruction::Call(0),
                        Instruction::LocalGet(0),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::Call(0),
                        Instruction::I32Add,
                        Instruction::Return,
                        Instruction::End,
                    ],
                }]),
                export_section: Some(vec![Export {
                    name: "fib".into(),
                    desc: ExportDesc::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }
}
