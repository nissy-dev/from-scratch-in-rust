use crate::parse::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum Instructions {
    DupPlusFP(i32),          // どういう命令...?
    MoveMinusFP(usize, i32), // どういう命令...?
    MovePlusFP(usize),       // どういう命令...?
    Store(i32),
    Return,
    JumpIfNotZero(String),
    Jump(String),
    Call(String, usize),
    Add,
    Subtract,
    LessThan,
}

// 関数やif文の範囲を表すために利用する
#[derive(Debug)]
struct Symbol {
    location: i32,
    num_arguments: usize,
    num_locals: usize,
}

#[derive(Debug)]
pub struct Program {
    syms: HashMap<String, Symbol>,
    instructions: Vec<Instructions>, // 命令
}

// ASTをPCへの命令に変換している
pub fn compile(raw: &[char], ast: Ast) -> Program {
    // ローカル変数の記録
    let mut locals: HashMap<String, i32> = HashMap::new();
    let mut pgm = Program {
        syms: HashMap::new(),
        instructions: Vec::new(),
    };

    for statement in ast {
        compile_statement(&mut pgm, raw, &mut locals, statement);
    }

    pgm
}

fn compile_statement(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    statement: Statement,
) {
    match statement {
        Statement::FunctionDeclaration(function_declaration) => {
            compile_declaration(program, raw, locals, function_declaration)
        }
        Statement::Return(return_) => compile_return(program, raw, locals, return_),
        Statement::If(if_) => compile_if(program, raw, locals, if_),
        Statement::Local(local_) => compile_local(program, raw, locals, local_),
        Statement::Expression(expression_) => compile_expression(program, raw, locals, expression_),
    }
}

fn compile_declaration(
    program: &mut Program,
    raw: &[char],
    _: &mut HashMap<String, i32>,
    function_declaration: FunctionDeclaration,
) {
    // jump to end of function to guard top-level
    let done_label = format!("function_done_{}", program.instructions.len());
    program
        .instructions
        .push(Instructions::Jump(done_label.clone()));

    let mut new_locals = HashMap::<String, i32>::new();
    let function_index = program.instructions.len() as i32;
    let num_arguments = function_declaration.parameters.len();
    for (i, parameter) in function_declaration.parameters.iter().enumerate() {
        program.instructions.push(Instructions::MoveMinusFP(
            i,
            num_arguments as i32 - (i as i32 + 1),
        ));
        new_locals.insert(parameter.value.clone(), i as i32);
    }

    for statement in function_declaration.body {
        compile_statement(program, raw, &mut new_locals, statement);
    }

    // 関数の始まりと終わりを記録
    program.syms.insert(
        function_declaration.name.value,
        Symbol {
            location: function_index as i32,
            num_arguments: num_arguments,
            num_locals: new_locals.keys().len(),
        },
    );

    program.syms.insert(
        done_label,
        Symbol {
            location: program.instructions.len() as i32,
            num_arguments: 0,
            num_locals: 0,
        },
    );
}

fn compile_local(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    local: Local,
) {
    let index = locals.keys().len();
    locals.insert(local.name.value, index as i32);
    compile_expression(program, raw, locals, local.expression);
    program.instructions.push(Instructions::MovePlusFP(index));
}

fn compile_literal(
    program: &mut Program,
    _: &[char],
    locals: &mut HashMap<String, i32>,
    literal: Literal,
) {
    match literal {
        Literal::Number(number) => {
            program
                .instructions
                .push(Instructions::Store(number.value.parse::<i32>().unwrap()));
        }
        Literal::Identifier(identifier) => {
            program
                .instructions
                .push(Instructions::DupPlusFP(locals[&identifier.value]));
        }
    }
}

fn compile_function_call(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    function_call: FunctionCall,
) {
    let length = function_call.arguments.len();
    for arg in function_call.arguments {
        compile_expression(program, raw, locals, arg);
    }

    program
        .instructions
        .push(Instructions::Call(function_call.name.value, length));
}

fn compile_binary_operation(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    binary_operation: BinaryOperation,
) {
    compile_expression(program, raw, locals, *binary_operation.left);
    compile_expression(program, raw, locals, *binary_operation.right);
    match binary_operation.operator.value.as_str() {
        "+" => {
            program.instructions.push(Instructions::Add);
        }
        "-" => {
            program.instructions.push(Instructions::Subtract);
        }
        "<" => {
            program.instructions.push(Instructions::LessThan);
        }
        _ => panic!(
            "{}",
            binary_operation
                .operator
                .location
                .debug(raw, "Unable to compile binary operation")
        ),
    }
}

fn compile_expression(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    expression: Expression,
) {
    match expression {
        Expression::BinaryOperation(binary_operation) => {
            compile_binary_operation(program, raw, locals, binary_operation);
        }
        Expression::FunctionCall(function_call) => {
            compile_function_call(program, raw, locals, function_call);
        }
        Expression::Literal(literal) => {
            compile_literal(program, raw, locals, literal);
        }
    }
}

fn compile_if(program: &mut Program, raw: &[char], locals: &mut HashMap<String, i32>, if_: If) {
    compile_expression(program, raw, locals, if_.test);
    let done_label = format!("if_else_{}", program.instructions.len());
    program
        .instructions
        .push(Instructions::JumpIfNotZero(done_label.clone()));

    for statement in if_.body {
        compile_statement(program, raw, locals, statement);
    }

    // If の始まりは登録しなくてよい...? -> if をスキップする時に使う
    program.syms.insert(
        done_label,
        Symbol {
            location: program.instructions.len() as i32 - 1,
            num_arguments: 0,
            num_locals: 0,
        },
    );
}

fn compile_return(
    program: &mut Program,
    raw: &[char],
    locals: &mut HashMap<String, i32>,
    return_: Return,
) {
    compile_expression(program, raw, locals, return_.expression);
    program.instructions.push(Instructions::Return);
}

pub fn eval(program: Program) {
    let mut pc: i32 = 0; // program counter
    let mut fp: i32 = 0; // frame pointer, 関数が呼び出される度に変化する

    let mut data: Vec<i32> = Vec::new(); // stack
    while pc < program.instructions.len() as i32 {
        match &program.instructions[pc as usize] {
            Instructions::Add => {
                let right = data.pop().unwrap();
                let left = data.pop().unwrap();
                data.push(left + right);
                pc += 1;
            }
            Instructions::Subtract => {
                let right = data.pop().unwrap();
                let left = data.pop().unwrap();
                data.push(left - right);
                pc += 1;
            }
            Instructions::LessThan => {
                let right = data.pop().unwrap();
                let left = data.pop().unwrap();
                data.push(if left < right { 1 } else { 0 });
                pc += 1;
            }
            Instructions::Store(value) => {
                data.push(*value);
                pc += 1;
            }
            Instructions::JumpIfNotZero(label) => {
                let top = data.pop().unwrap();
                if top == 0 {
                    // もし条件式が false ならば、処理をスキップする
                    pc = program.syms[label].location;
                }
                pc += 1;
            }
            Instructions::Jump(label) => {
                pc = program.syms[label].location;
            }
            // Loading from a variable
            // The MovePlusFP instruction copies a value from the stack (offset the frame pointer) onto the top of the stack.
            // This is for references to arguments and locals.
            // ローカル変数の読み込み (DupPlusFPが fp+offset で取りに行くから、それに合わせた形で積んでいる...?)
            Instructions::MovePlusFP(offset) => {
                let value = data.pop().unwrap();
                let index = fp as usize + *offset;
                // Account for top-level locals
                while index >= data.len() {
                    data.push(0);
                }
                data[index] = value;
                pc += 1;
            }
            // Storing locals
            // The DupPlusFP instruction is used by compile_locals
            // to store a local once compiled onto the stack in the relative position from the frame pointer.
            // すでに読み込まれたローカル変数をコピーしてくる
            Instructions::DupPlusFP(offset) => {
                data.push(data[(fp + offset) as usize]);
                pc += 1;
            }
            // Duplicating arguments
            // The MoveMinusFP instruction is, again, a hack to work around limited addressing modes in this minimal virtual machine.
            // It copies arguments from behind the frame pointer to in front of the frame pointer.
            // 関数の呼び出しで用意した領域に引数をコピーしてくる
            Instructions::MoveMinusFP(local_offset, fp_offset) => {
                data[fp as usize + local_offset] = data[(fp - (fp_offset + 4)) as usize];
                pc += 1;
            }
            // Calling a function
            // frame pointer, program counter, 引数の数, 変数と引数 (領域のみ) をスタックに積む
            Instructions::Call(label, num_arguments) => {
                if label == "print" {
                    for _ in 0..*num_arguments {
                        let value = data.pop().unwrap();
                        print!("{}", value);
                        print!(" ");
                    }
                    println!();
                    pc += 1;
                    continue;
                }

                data.push(fp);
                data.push(pc + 1);
                data.push(program.syms[label].num_arguments as i32);
                pc = program.syms[label].location;
                fp = data.len() as i32;

                // set up space for all arguments and locals
                let mut num_locals = program.syms[label].num_locals;
                while num_locals > 0 {
                    data.push(0);
                    num_locals -= 1;
                }
            }
            Instructions::Return => {
                let return_value = data.pop().unwrap();

                // clean up the local stack
                while fp < data.len() as i32 {
                    data.pop();
                }

                // restore pc and fp
                let mut num_arguments = data.pop().unwrap();
                pc = data.pop().unwrap();
                fp = data.pop().unwrap();

                // clean up the argument stack
                while num_arguments > 0 {
                    data.pop();
                    num_arguments -= 1;
                }

                data.push(return_value);
            }
        }
    }
}
