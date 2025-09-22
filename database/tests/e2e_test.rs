use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdout, Command, Stdio};
use std::vec;

struct TestProcess {
    db_file_path: String,
}

impl TestProcess {
    fn new(db_file_path: &str) -> Self {
        TestProcess {
            db_file_path: db_file_path.into(),
        }
    }

    fn run_script(&mut self, commands: Vec<String>) -> Vec<String> {
        let mut child = Command::new("target/debug/database")
            .arg(&self.db_file_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start database process");
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        // スキーマはあらかじめ作っておく
        let mut all_commands = vec!["create int text(32) text(255)".into()];
        all_commands.extend(commands);
        // 終了コマンドを追加
        all_commands.push(".exit".into());

        for command in all_commands {
            // コマンド送信
            writeln!(stdin, "{}", command).expect("Failed to write to stdin");
            stdin.flush().expect("Failed to flush stdin");
        }

        // コマンドの結果を収集
        let stdout: BufReader<&mut ChildStdout> =
            BufReader::new(child.stdout.as_mut().expect("Failed to open stdout"));
        let mut output = Vec::new();
        for line in stdout.lines() {
            output.extend(
                line.expect("Failed to read line")
                    .split("db > ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
        }
        output
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db_file_path);
    }
}

#[test]
fn test_insert_and_select() {
    let mut process = TestProcess::new("./test.db");
    let commands = vec!["insert 1 user1 person1@example.com".into(), "select".into()];
    let results = process.run_script(commands);
    println!("results {:?}", results);
    assert!(results[0].contains("(1, user1, person1@example.com)"))
}

#[test]
fn test_insert_max_length() {
    let mut process = TestProcess::new("./test.db");
    let long_username = "a".repeat(32);
    let long_email = "a".repeat(255);
    let commands = vec![
        format!("insert 1 {} {}", long_username, long_email),
        "select".into(),
    ];
    let results = process.run_script(commands);
    assert!(results[0].contains(&format!("(1, {}, {})", long_username, long_email)));
}

#[test]
fn test_insert_invalid_max_length() {
    let mut process = TestProcess::new("./test.db");
    let long_username = "a".repeat(33);
    let long_email = "a".repeat(256);
    let commands = vec![format!("insert 1 {} {}", long_username, long_email)];
    let results = process.run_script(commands);
    assert!(results[0].contains("Error: Failed to validate row"));
}

#[test]
fn test_table_full() {
    let mut process = TestProcess::new("./test.db");
    let commands = (1..1402)
        .map(|i| format!("insert {} user{} person{}@example.com", i, i, i))
        .collect::<Vec<_>>();
    let results = process.run_script(commands);
    assert!(results[0].contains("Error: Table is full"));
}

// #[test]
// fn test_data_persistence() {
//     let mut process = TestProcess::new("./test.db");

//     let commands = vec!["insert 1 user1 person1@example.com".into()];
//     process.run_script(commands);

//     let commands = vec!["select".into()];
//     let results = process.run_script(commands);
//     assert!(results[0].contains("(1, user1, person1@example.com)"));
// }
