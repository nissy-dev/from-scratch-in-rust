mod backend;
mod frontend;
mod schema;

use std::{env::args, io::Write};

use anyhow::{Error, Result};

use crate::frontend::{InputBuffer, MetaCommand, Statement};

fn main() -> Result<(), Error> {
    let args: Vec<_> = args().collect();
    if args.len() < 2 {
        println!("Must supply a database filename.");
        std::process::exit(1);
    }
    let mut input = InputBuffer::new();
    // 複数テーブルサポートはしない
    let mut table = backend::Table::new(&args[1])?;
    loop {
        print!("db > ");
        std::io::stdout().flush().unwrap();
        input.read_input()?;

        if input.is_meta_command() {
            let command = MetaCommand::new(&input.buffer);
            command.execute()?;
            continue;
        }

        let statement = Statement::new(&input.buffer);
        if let Err(err) = statement.execute(&mut table) {
            println!("Error: {}", err);
            table.reset()?;
        }
    }
}
