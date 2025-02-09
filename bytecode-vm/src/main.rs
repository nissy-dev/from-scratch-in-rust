use std::io::Write;

mod compiler;
mod lexer;
mod parser;
mod token;
mod value;
mod vm;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 3 {
        tracing::info!("Usage: clox [script]");
        std::process::exit(1);
    } else if args.len() == 2 {
        let source = std::fs::read_to_string(&args[1]).expect("Failed to read file");
        run(source);
    } else {
        loop {
            print!("> ");
            std::io::stdout().flush().unwrap();

            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            println!("You entered: {}", line);
            run(line);
        }
    }
}

fn run(source: String) {
    let mut compiler = compiler::Compiler::new(source);
    match compiler.compile() {
        Ok(codes) => {
            println!("OpCodes: {:?}", codes);
            let mut vm = vm::VirtualMachine::new(codes);
            match vm.interpret() {
                Ok(_) => {
                    tracing::info!("Interpretation completed");
                    println!("ObjectList: {:?}", vm.object_list);
                }
                Err(error) => {
                    tracing::error!("Error: {:?}", error);
                }
            }
        }
        Err(error) => {
            tracing::error!("Error: {:?}", error);
        }
    }
}
