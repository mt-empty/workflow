use anyhow::{Error as AnyError, Result};
use clap::{Parser, Subcommand};
use diesel::{PgConnection, QueryDsl, RunQueryDsl, Selectable, SelectableHelper, Table};
use prettytable::{row, Cell, Row, Table as PrettyTable};
use serde_json::Value;
use std::fs::File;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::field;
use workflow::engine::run_task_process;
use workflow::engine::{handle_stop, run_event_process};
use workflow::models::{Engine, Event, Task};
use workflow::parser::process_yaml_file;
use workflow::utils::establish_pg_connection;
use workflow::utils::run_migrations;

// #[clap(about = "A tool to command workflow engine", author, version)]
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
    // Lists that takes subcommands, such as `list tasks` or `list events` or `list engines` or `list workflows` or `list all`
    List {
        #[clap(subcommand)]
        subcommand: ListSubcommands,
    },
}

#[derive(Subcommand)]
enum ListSubcommands {
    // Lists all tasks
    Tasks {},
    // Lists all events
    Events {},
    // Lists all engines
    Engines {},
    // Lists all workflows
    Workflows {},
    // Lists all
    All {},
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
        }
        Commands::Setup {} => {
            println!("Setup");

            if let Err(e) = run_migrations() {
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
            //todo: handle stop for multiple engines
            if let Err(e) = handle_stop() {
                println!("Failed to stop the engine, {}", e);
                std::process::exit(1);
            };
        }
        Commands::Add { file_path } => {
            println!("Adding file: {}", file_path);
            if let Err(e) = process_yaml_file(file_path.to_string()) {
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
        Commands::List { subcommand } => {
            fn process_subcommand(subcommand: &ListSubcommands) {
                match subcommand {
                    ListSubcommands::Tasks {} => {
                        println!("Listing tasks");

                        let mut conn = establish_pg_connection();
                        let items = workflow::schema::tasks::dsl::tasks
                            .select(Task::as_select())
                            .load::<Task>(&mut conn)
                            .unwrap();
                        if let Err(e) = list_items(items) {
                            println!("Failed to add file, {}", e);
                            std::process::exit(1);
                        }
                    }
                    ListSubcommands::Events {} => {
                        println!("Listing events");

                        let mut conn = establish_pg_connection();
                        let items = workflow::schema::events::dsl::events
                            .select(Event::as_select())
                            .load::<Event>(&mut conn)
                            .unwrap();
                        if let Err(e) = list_items(items) {
                            println!("Failed to add file, {}", e);
                            std::process::exit(1);
                        }
                    }
                    ListSubcommands::Engines {} => {
                        println!("Listing engines");

                        let mut conn = establish_pg_connection();
                        let items = workflow::schema::engines::dsl::engines
                            .select(Engine::as_select())
                            .load::<Engine>(&mut conn)
                            .unwrap();
                        if let Err(e) = list_items(items) {
                            println!("Failed to add file, {}", e);
                            std::process::exit(1);
                        }
                    }
                    ListSubcommands::Workflows {} => {
                        println!("Listing workflows");
                    }
                    ListSubcommands::All {} => {
                        println!("Listing all");
                        process_subcommand(&ListSubcommands::Tasks {});
                        process_subcommand(&ListSubcommands::Events {});
                        process_subcommand(&ListSubcommands::Engines {});
                        process_subcommand(&ListSubcommands::Workflows {});
                    }
                }
            }
            process_subcommand(subcommand);
        }
    }
    std::process::exit(0);
}

// fn list<T>(any_table: &T) -> Result<(), diesel::result::Error>
// where
//     T: Table,
// {
//     let mut conn = establish_pg_connection();
//     let columns = T::all_columns;
//     let items: Vec<T> = any_table.select(tasks::all_columns).load(&mut conn)?;
//     list_items(&items);
//     Ok(())
// }

fn list_items<T: serde::ser::Serialize>(items: Vec<T>) -> Result<(), AnyError> {
    let mut pretty_table = PrettyTable::new();

    let binding = serde_json::to_value(&items[0]).expect("Failed to serialize vector to JSON");
    let fields: Vec<String> = binding
        .as_object()
        .expect("Expected JSON object")
        .keys()
        .cloned()
        .collect();

    // Add the table headers
    pretty_table.add_row(fields.iter().map(|field| field.to_string()).collect());

    // Add each item to the table
    for item in items {
        let values: Vec<Value> = serde_json::to_value(&item)
            .expect("Failed to serialize vector to JSON")
            .as_object()
            .expect("Expected JSON object")
            .values()
            .cloned()
            .collect();

        pretty_table.add_row(
            values
                .iter()
                .map(|value| -> String {
                    let mut s = value.to_string();
                    s.truncate(50);
                    format!("{}...", s)
                })
                .collect(),
        );
    }

    pretty_table.printstd();
    Ok(())
}

// fn is_redis_running() -> bool {
//     let redis_result = create_redis_connection();
//     if let Err(e) = redis_result {
//         eprintln!("Failed to connect to redis {}", e);
//         return false;
//     }
//     true
// }

// fn is_postgres_running() -> bool {
//     let mut client = ();
//     if let Err(e) = client {
//         eprintln!("Failed to create a postgres client {}", e);
//         return false;
//     }
//     true
// }

// fn is_engine_running() -> bool {
//     is_redis_running() && is_postgres_running()
// }
