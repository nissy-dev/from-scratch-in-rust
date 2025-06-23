mod eval;
mod lex;
mod parse;

use std::env;
use std::fs;

// entrypoint
fn main() {
    let args: Vec<String> = env::args().collect();
    let contents = fs::read_to_string(&args[1]).expect("Could not read file");

    let raw: Vec<char> = contents.chars().collect();

    // lexer: 字句解析
    // perform lexical analysis to retrieve all tokens from the file
    let tokens = match lex::lex(&raw) {
        Ok(tokens) => tokens,
        Err(msg) => panic!("{}", msg),
    };

    // parser: 構文解析
    // perform grammar analysis on the tokens to retrieve a tree structure
    let ast = match parse::parse(&raw, tokens) {
        Ok(ast) => ast,
        Err(msg) => panic!("{}", msg),
    };

    // compiler: コンパイル
    // compile the tree to a linear set of virtual machine instructions
    let pgm = eval::compile(&raw, ast);

    // evaluator: 評価
    // interpret the virtual machine instructions
    eval::eval(pgm);
}
