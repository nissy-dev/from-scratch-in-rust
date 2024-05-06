use std::{
    env,
    error::Error,
    io::{Read, Write},
    net::TcpListener,
    str, thread,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<String>>();
    let addr = &args[1];
    echo_server(addr)?;
    Ok(())
}

fn echo_server(addr: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr)?;
    loop {
        let (mut stream, _) = listener.accept()?;
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];

            loop {
                let nbytes = stream.read(&mut buffer).unwrap();
                if nbytes == 0 {
                    return;
                }
                println!("{:?}", str::from_utf8(&buffer[..nbytes]));
                stream.write_all(&buffer[..nbytes]).unwrap();
            }
        });
    }
}
