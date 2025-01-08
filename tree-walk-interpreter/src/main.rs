use std::{cell::RefCell, io::Write, rc::Rc};

mod ast;
mod interpreter;
mod lexer;
mod parser;
mod resolver;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 3 {
        tracing::info!("Usage: lox [script]");
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
    let mut scanner = lexer::Scanner::new(source);
    scanner.scan();
    let mut parser = parser::Parser::new(scanner.tokens);
    let stmts = parser.parse();
    let interpreter = Rc::new(RefCell::new(interpreter::Interpreter::new()));
    let mut resolver = resolver::Resolver::new(interpreter.clone());
    match stmts {
        Ok(stmts) => {
            resolver.resolve(stmts.clone());
            print!("{:?}", interpreter.borrow());
            let result = interpreter.borrow_mut().interpret(stmts);
            match result {
                Ok(_) => {}
                Err(e) => tracing::error!("{:?}", e),
            }
        }
        Err(e) => tracing::error!("{:?}", e),
    }
}
