use anyhow::Error as AnyError;

mod cli;

fn main() -> Result<(), AnyError> {
    cli::cli();
    Ok(())
}
