use anyhow::Error as AnyError;

mod cli;

fn main() -> Result<(), AnyError> {
    println!("Hello, world!");
    cli::cli();

    Ok(())
}
