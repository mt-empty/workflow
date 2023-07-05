use anyhow::{Error as AnyError, Ok, Result};
use clap::{Parser, Subcommand};
use std::process::Command;
use std::{
    fs::{read_to_string, File},
    io::Read,
};

use workflow::engine::handle_start;
use workflow::engine::handle_stop;
use workflow::parser::parse;

use std::collections::BTreeMap;
// use std::io::{BufRead, BufReader, Cursor, Read};
// use std::path::{Path, PathBuf};
// mod utils;
// use utils::get_redis_con;

// const PAGES_BASE_DIR: &str = "";

// #[clap(about = "A tool to manipulate  workflow tasks", author, version)]
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Starts the engine
    Start {},
    // Stops the engine
    Engine {},
    Stop {},
    /// Adds workflow to the queue
    Add {
        file_path: String,
    },
    // Shows the status of a task
    Show {
        task_name: String,
    },
    // Pauses the task
    Pause {
        task_name: String,
    },
    // Continues the task
    Continue {
        task_name: String,
    },
    // Aborts the task
    Abort {
        task_name: String,
    },
    // Lists all tasks
    List,
}

pub fn cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start {} => {
            println!("Starting the engine");
            let stdout_file =
                File::create("engine_stdout.txt").expect("Failed to create stdout file");
            // delete the content of the file
            let _ = std::fs::write("engine_stdout.txt", "");
            let stderr_file =
                File::create("engine_stderr.txt").expect("Failed to create stderr file");
            let _ = std::fs::write("engine_stderr.txt", "");
            Command::new("cargo")
                .arg("run")
                // .arg("--bin")
                .arg("engine")
                .stderr(stderr_file)
                .stdout(stdout_file)
                .spawn()
                .expect("Failed to start engine");
            std::process::exit(0);
        }
        Commands::Engine {} => {
            println!("Engine");
            if let Err(e) = handle_start() {
                println!("Failed to start the engine, {}", e);
                std::process::exit(1);
            };
        }
        Commands::Stop {} => {
            println!("Stopping the engine");
            if let Err(e) = handle_stop() {
                println!("Failed to stop the engine, {}", e);
                std::process::exit(1);
            };
            std::process::exit(0);
        }
        Commands::Add { file_path } => {
            println!("Adding file: {}", file_path);
            if let Err(e) = parse(file_path.to_string()) {
                println!("Failed to add file, {}", e);
                std::process::exit(1);
            }
            // Add your logic for the 'add' Commands here
        }
        Commands::Show { task_name } => {
            println!("Showing task: {}", task_name);
            // Add your logic for the 'show' Commands here
        }
        Commands::Pause { task_name } => {
            println!("Continuing task: {}", task_name);
            // Add your logic for the 'continue' Commands here
        }
        Commands::Continue { task_name } => {
            println!("Continuing task: {}", task_name);
            // Add your logic for the 'continue' Commands here
        }
        Commands::Abort { task_name } => {
            println!("Aborting task: {}", task_name);
            // Add your logic for the 'abort' Commands here
        }
        Commands::List => {
            println!("Listing all tasks");
            // Add your logic for the 'list' Commands here
        }
    }
}
