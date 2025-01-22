use anyhow::Result;
use clap::Parser;

mod input;
mod job;
mod r#match;

fn main() -> Result<()> {
    job::Job::parse().execute()
}
