use std::io::Write as _;

use anyhow::Result;

pub fn input(prompt: &str, multiline: bool) -> Result<String> {
    print!("{}> ", prompt);
    std::io::stdout().flush()?;
    let stdin = std::io::stdin();
    let mut buf = String::new();
    if multiline {
        println!();
        while stdin.read_line(&mut buf)? > 1 {}
    } else {
        stdin.read_line(&mut buf)?;
    }
    while matches!(buf.as_bytes().last(), Some(b'\n' | b'\r')) {
        buf.pop();
    }
    Ok(buf)
}
