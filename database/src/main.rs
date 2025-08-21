mod frontend;

use std::io::Write;

use crate::frontend::{InputBuffer, MetaCommand, Statement};

fn main() {
    let mut input = InputBuffer::new();
    loop {
        print!("db > ");
        std::io::stdout().flush().unwrap();

        input.read_input();

        if input.is_meta_command() {
            let command = MetaCommand::new(&input.buffer);
            command.execute();
            continue;
        }

        let statement = Statement::new(&input.buffer);

        // if input.is_meta_command() {
        //     if input.buffer == ".exit" {
        //         break;
        //     } else {
        //         println!("Unrecognized command '{}'.", input.buffer);
        //     }
        // }
    }
}
