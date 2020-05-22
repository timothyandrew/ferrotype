use std::io::{self, Write};

pub fn read_cli_input(message: &str) -> String {
    println!("{}", message);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    buf.trim().to_owned()
}
