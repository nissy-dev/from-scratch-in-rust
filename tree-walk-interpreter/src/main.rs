use std::io::Write;

mod lexer;

fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 3 {
        tracing::info!("Usage: jlox [script]");
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
    for token in scanner.tokens {
        tracing::info!("{:?}", token);
    }
}
