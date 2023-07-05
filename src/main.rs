use anyhow::Error as AnyError;

mod cli;
mod parser;

fn main() -> Result<(), AnyError> {
    println!("Hello, world!");
    cli::cli();

    Ok(())
}
