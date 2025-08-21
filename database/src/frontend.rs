pub struct InputBuffer {
    pub buffer: String,
    pub length: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        InputBuffer {
            buffer: String::new(),
            length: 0,
        }
    }

    pub fn read_input(&mut self) {
        let mut input = String::new();
        self.length = std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        self.buffer.push_str(input.trim());
    }

    pub fn is_meta_command(&self) -> bool {
        self.buffer.starts_with('.')
    }
}

pub struct MetaCommand<'a> {
    command: &'a str,
}

impl<'a> MetaCommand<'a> {
    pub fn new(command: &'a str) -> Self {
        MetaCommand { command }
    }

    pub fn execute(&self) {
        match self.command {
            ".exit" => {
                std::process::exit(0);
            }
            _ => {
                println!("Unrecognized command '{}'.", self.command);
            }
        }
    }
}

pub struct Statement<'a> {
    content: &'a str,
}

impl<'a> Statement<'a> {
    pub fn new(content: &'a str) -> Self {
        Statement { content }
    }

    pub fn execute(&self) {
        // Placeholder for statement execution logic
        println!("Executing statement: {}", self.content);
    }
}
