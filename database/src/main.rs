mod backend;
mod frontend;
mod schema;

use std::io::Write;

use anyhow::{Error, Result};

use crate::frontend::{InputBuffer, MetaCommand, Statement};

fn main() -> Result<(), Error> {
    let mut input = InputBuffer::new();
    // 複数テーブルサポートはしない
    let mut table = backend::Table::new();
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
            println!("Error executing statement: {}", err);
            table.reset();
        }
    }
}
