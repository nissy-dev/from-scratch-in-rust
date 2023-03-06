use std::collections::HashMap;
use std::str;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type ExpireDate = Option<SystemTime>;

struct InMemoryDb {
    mutex: Mutex<HashMap<String, (Vec<u8>, ExpireDate)>>,
}

impl InMemoryDb {
    fn new() -> Self {
        Self {
            mutex: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, key: String) -> Option<(Vec<u8>, ExpireDate)> {
        let lock = self.mutex.lock().unwrap();
        Some(lock.get(&key)?.clone())
    }

    fn insert(&self, key: String, value: (Vec<u8>, ExpireDate)) {
        let mut lock = self.mutex.lock().unwrap();
        lock.insert(key, value);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    let db: Arc<InMemoryDb> = Arc::new(InMemoryDb::new());

    loop {
        let (socket, _) = listener.accept().await?;
        let db = db.clone();
        tokio::spawn(async move {
            handle_connection(socket, db).await.unwrap();
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    db: Arc<InMemoryDb>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];

    loop {
        let n_bytes = stream.read(&mut buffer).await?;
        let receive_str = str::from_utf8(&buffer[..n_bytes])?;
        let lines = receive_str
            .split("\r\n")
            .into_iter()
            .filter_map(|item| match item.chars().next()? {
                '$' | '*' => None,
                _ => Some(item),
            })
            .collect::<Vec<_>>();

        if lines.len() == 0 {
            break;
        }

        match lines[0].to_uppercase().as_str() {
            "PING" => {
                if lines.len() == 1 {
                    let send_str = "+PONG\r\n";
                    stream.write(send_str.as_bytes()).await?;
                }
            }
            "ECHO" => {
                if lines.len() == 2 {
                    let send_str = format!("${}\r\n{}\r\n", lines[1].len(), lines[1]);
                    stream.write(send_str.as_bytes()).await?;
                }
            }
            "SET" => match lines.len() {
                3 => {
                    db.insert(lines[1].to_string(), (lines[2].as_bytes().to_vec(), None));
                    let send_str = "+OK\r\n";
                    stream.write(send_str.as_bytes()).await?;
                }
                5 => {
                    let expire_date = SystemTime::now()
                        .checked_add(Duration::from_millis(lines[4].parse::<u64>().unwrap()));
                    db.insert(
                        lines[1].to_string(),
                        (lines[2].as_bytes().to_vec(), expire_date),
                    );
                    let send_str = "+OK\r\n";
                    stream.write(send_str.as_bytes()).await?;
                }
                _ => {
                    println!("Error: wrong number of arguments for 'set' command");
                }
            },
            "GET" => {
                if lines.len() != 2 {
                    println!("Error: wrong number of arguments for 'get' command");
                    continue;
                }

                let mut send_str = String::from("$-1\r\n");
                if let Some((value, expire_date)) = db.get(lines[1].to_string()) {
                    let value = str::from_utf8(&value)?;
                    send_str = format!("${}\r\n{}\r\n", value.len(), value);
                    if let Some(expire_date) = expire_date {
                        if expire_date < SystemTime::now() {
                            send_str = String::from("$-1\r\n");
                        }
                    }
                }
                stream.write(send_str.as_bytes()).await?;
            }
            _ => {
                println!("Unsupported command: {:?}", lines[0]);
                break;
            }
        }
    }

    Ok(())
}
