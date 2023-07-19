use anyhow::{Error as AnyError, Ok, Result};
use clap::{Parser, Subcommand};
use std::os::unix::process;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::{
    fs::{read_to_string, File},
    io::Read,
};

use workflow::engine::{handle_stop, run_event_process};
use workflow::engine::{initialize_tables, run_task_process};
use workflow::parser::process;
use workflow::utils::create_postgres_client;
use workflow::utils::create_redis_connection;
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
    Start {
        // add detach flag
        #[clap(short, long)]
        detach: bool,
    },
    // Stops the engine
    Setup {},
    StartTaskProcess {},
    StartEventProcess {},
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

#[derive(PartialEq)]
enum ProcessType {
    Setup,
    Task,
    Event,
}

fn create_and_clear_log_file(file_path: &str) -> Result<File, AnyError> {
    let _ = std::fs::create_dir("./logs");
    let file = File::create(file_path)?;
    let _ = std::fs::write(file_path, "");
    Ok(file)
}

fn start_process(subcommand_name: &str, process_type: ProcessType) -> Result<(), AnyError> {
    let _ = std::fs::create_dir("./logs");

    let stdout_path = match process_type {
        ProcessType::Setup => "./logs/setup_stdout.txt",
        ProcessType::Task => "./logs/task_stdout.txt",
        ProcessType::Event => "./logs/event_stdout.txt",
    };
    let stderr_path = match process_type {
        ProcessType::Setup => "./logs/setup_stderr.txt",
        ProcessType::Task => "./logs/task_stderr.txt",
        ProcessType::Event => "./logs/event_stderr.txt",
    };
    let stdout = create_and_clear_log_file(stdout_path)?;
    let stderr = create_and_clear_log_file(stderr_path)?;

    let mut binding = Command::new("cargo");
    let command = binding
        .arg("run")
        // .arg("--bin")
        .arg(subcommand_name)
        .stdout(stdout)
        .stderr(stderr);

    let mut child = command.spawn()?;

    if process_type == ProcessType::Setup {
        let _ = child.wait()?;
    }
    Ok(())
}

pub fn cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { detach } => {
            println!("Starting the Engine");

            if let Err(e) = start_process("setup", ProcessType::Setup) {
                eprintln!("Failed to start Setup process: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }

            if let Err(e) = start_process("start-event-process", ProcessType::Event) {
                eprintln!("Failed to start Event process: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }

            if let Err(e) = start_process("start-task-process", ProcessType::Task) {
                eprintln!("Failed to start Task process: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }

            println!("Engine started successfully");
            std::process::exit(0);
        }
        Commands::Setup {} => {
            println!("Setup");

            if let Err(e) = initialize_tables() {
                eprintln!("Failed to create initial tables: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }
        }
        Commands::StartEventProcess {} => {
            println!("StartEventProcess");
            if let Err(e) = run_event_process() {
                println!("Failed to start event process, {}", e);
                std::process::exit(1);
            };
        }
        Commands::StartTaskProcess {} => {
            println!("StartTaskProcess");
            if let Err(e) = run_task_process() {
                println!("Failed to start task process, {}", e);
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
            if let Err(e) = process(file_path.to_string()) {
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

fn is_redis_running() -> bool {
    let redis_result = create_redis_connection();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        return false;
    }
    true
}

fn is_postgres_running() -> bool {
    let mut client = create_postgres_client();
    if let Err(e) = client {
        eprintln!("Failed to create a postgres client {}", e);
        return false;
    }
    true
}

fn is_engine_running() -> bool {
    is_redis_running() && is_postgres_running()
}
