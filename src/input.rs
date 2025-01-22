use std::io::Write as _;

use anyhow::Result;
use crossterm::style::{style, Attribute, Stylize};

pub fn input(prompt: &str, multiline: bool) -> Result<String> {
    print!(
        "{} ",
        style(format_args!("{}> ", prompt))
            .attribute(Attribute::Bold)
            .attribute(Attribute::Dim)
    );
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
