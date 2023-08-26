use anyhow::{anyhow, Error as AnyError, Result};
use clap::{Parser, Subcommand};
use diesel::{ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use dotenv::dotenv;
use pnet::datalink::interfaces;
use prettytable::{Cell, Row, Table as PrettyTable};
use serde_json::Value;
use std::env;
use std::fs::File;
use std::process::Command;
use tracing::field;
use workflow::engine_utils::{create_new_engine_entry, handle_stop, update_engine_status};
use workflow::models::{Engine, EngineStatus, Event, Task};
use workflow::parser::process_yaml_file;
use workflow::utils::establish_pg_connection;
use workflow::utils::run_migrations;

const PRETTY_TABLE_MAX_CELL_LEN: usize = 50;
const ENGINE_NAME: &str = "workflow-engine";
const ENGINE_IP_ADDRESS: &str = "0.0.0.0";

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
    Start {},
    // Stops the engine
    Migration {},
    StartTaskProcess {
        engine_uid: i32,
    },
    StartEventProcess {
        engine_uid: i32,
    },
    Stop {},
    /// Adds workflow to the queue
    Add {
        file_path: String,
    },
    // Shows the status of a task
    Show {
        #[clap(subcommand)]
        subcommand: ShowSubcommands,
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
    // Lists all workflows
    Workflows {},
    // Lists all engines
    Engines {},
    // Lists all
    All {},
}

#[derive(Subcommand)]
enum ShowSubcommands {
    // Lists all tasks
    Task { uid: i32 },
    // Lists all events
    Event { uid: i32 },
    // Lists all workflows
    Workflow { uid: i32 },
    // Lists all engines
    Engine { uid: i32 },
}

#[derive(PartialEq)]
enum ProcessType {
    Task,
    Event,
}

fn create_and_clear_log_file(file_path: &str) -> Result<File, AnyError> {
    let file = File::create(file_path)?;
    let _ = std::fs::write(file_path, "");
    Ok(file)
}

fn start_process(
    binary_name: &str,
    process_type: ProcessType,
    engine_uid: i32,
) -> Result<(), AnyError> {
    let _ = std::fs::create_dir("./logs");

    let stdout_path = match process_type {
        ProcessType::Task => "./logs/task_stdout.txt",
        ProcessType::Event => "./logs/event_stdout.txt",
    };
    let stderr_path = match process_type {
        ProcessType::Task => "./logs/task_stderr.txt",
        ProcessType::Event => "./logs/event_stderr.txt",
    };
    let stdout = create_and_clear_log_file(stdout_path)?;
    let stderr = create_and_clear_log_file(stderr_path)?;

    dotenv().ok();

    let env_var = env::var("ENVIRONMENT").unwrap_or("dev".to_owned());
    let is_production = env_var == "prod";

    let mut binding;
    if is_production {
        binding = Command::new("./workflow");
    } else {
        binding = Command::new("cargo");
        binding.arg("run");
        // Add other arguments for development environment if needed
    }

    let command = binding
        .arg("--bin")
        .arg(binary_name)
        .arg(engine_uid.to_string())
        .stdout(stdout)
        .stderr(stderr);

    let mut _child = command.spawn()?;
    Ok(())
}

pub fn cli() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start {} => {
            println!("Starting the Engine");
            if let Err(e) = process_start_command() {
                eprintln!("Failed to start the engine: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }
        }
        Commands::Migration {} => {
            println!("Migration");

            if let Err(e) = run_migrations() {
                eprintln!("Failed to run DB migrations: {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }
        }
        Commands::StartEventProcess { engine_uid } => {
            println!("StartEventProcess");
            // if let Err(e) = run_event_process(*engine_uid) {
            //     println!("Failed to start event process, {}", e);
            //     std::process::exit(1);
            // };
        }
        Commands::StartTaskProcess { engine_uid } => {
            println!("StartTaskProcess");
            // if let Err(e) = run_task_process(*engine_uid) {
            //     println!("Failed to start task process, {}", e);
            //     std::process::exit(1);
            // };
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
        }
        Commands::Show { subcommand } => {
            if let Err(e) = process_show_subcommands(&mut establish_pg_connection(), subcommand) {
                println!("Failed to show, {}", e);
                std::process::exit(1);
            };
        }
        Commands::Pause { task_name } => {
            println!("Continuing task: {}", task_name);
            todo!()
        }
        Commands::Continue { task_name } => {
            println!("Continuing task: {}", task_name);
            todo!()
        }
        Commands::Abort { task_name } => {
            println!("Aborting task: {}", task_name);
            todo!()
        }
        Commands::List { subcommand } => {
            if let Err(e) = process_list_subcommands(&mut establish_pg_connection(), subcommand) {
                println!("Failed to list, {}", e);
                std::process::exit(1);
            };
        }
    }
    std::process::exit(0);
}

fn get_system_ip_address() -> Result<String, AnyError> {
    // Get a vector with all network interfaces found
    let all_interfaces = interfaces();

    // Search for the default interface - the one that is
    // up, not loopback and has an IP.
    let default_interface = all_interfaces
        .iter()
        .find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty());

    match default_interface {
        Some(interface) => Ok(interface.ips[0].ip().to_string()),
        None => Err(anyhow!("No default interface found")),
    }
}

fn process_start_command() -> Result<(), AnyError> {
    use dotenv::dotenv;
    if let Err(e) = run_migrations() {
        eprintln!("Failed to run DB migrations: {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("DB migrations completed successfully");
    let conn = &mut establish_pg_connection();

    let engine_uid = create_new_engine_entry(
        conn,
        &env::var("ENGINE_NAME").unwrap_or(ENGINE_NAME.to_owned()),
        &get_system_ip_address()?,
    )?;
    println!("created new engine entry with uid: {}", engine_uid);

    if let Err(e) = start_process("event", ProcessType::Event, engine_uid) {
        eprintln!("Failed to start Event process: {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }

    if let Err(e) = start_process("task", ProcessType::Task, engine_uid) {
        eprintln!("Failed to start Task process: {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }

    update_engine_status(conn, engine_uid, EngineStatus::Running)?;

    println!("Engine started successfully");
    Ok(())
}

fn process_show_subcommands(
    conn: &mut PgConnection,
    subcommand: &ShowSubcommands,
) -> Result<(), AnyError> {
    match subcommand {
        ShowSubcommands::Task { uid } => {
            println!("Showing task: {}", uid);
            let item: Task = workflow::schema::tasks::dsl::tasks
                .select(Task::as_select())
                .filter(workflow::schema::tasks::dsl::uid.eq(uid))
                .first::<Task>(conn)?;
            list_items(vec![item])?
        }
        ShowSubcommands::Event { uid } => {
            println!("Showing event: {}", uid);
            let item: Event = workflow::schema::events::dsl::events
                .select(Event::as_select())
                .filter(workflow::schema::events::dsl::uid.eq(uid))
                .first::<Event>(conn)?;
            list_items(vec![item])?
        }
        ShowSubcommands::Workflow { uid } => {
            println!("Showing workflow: {}", uid);
            todo!()
        }
        ShowSubcommands::Engine { uid } => {
            println!("Showing engine: {}", uid);
            let item = workflow::schema::engines::dsl::engines
                .select(Engine::as_select())
                .filter(workflow::schema::engines::dsl::uid.eq(uid))
                .first::<Engine>(conn)?;
            list_items(vec![item])?
        }
    }
    Ok(())
}

fn process_list_subcommands(
    conn: &mut PgConnection,
    subcommand: &ListSubcommands,
) -> Result<(), AnyError> {
    match subcommand {
        ListSubcommands::Tasks {} => {
            println!("Listing tasks");
            let items = workflow::schema::tasks::dsl::tasks
                .select(Task::as_select())
                .load::<Task>(conn)?;
            list_items(items)
        }
        ListSubcommands::Events {} => {
            println!("Listing events");
            let items = workflow::schema::events::dsl::events
                .select(Event::as_select())
                .load::<Event>(conn)?;
            list_items(items)
        }
        ListSubcommands::Workflows {} => {
            println!("Listing workflows, not implemented yet");
            todo!()
        }
        ListSubcommands::Engines {} => {
            println!("Listing engines");
            let items = workflow::schema::engines::dsl::engines
                .select(Engine::as_select())
                .load::<Engine>(conn)?;
            list_items(items)
        }
        ListSubcommands::All {} => {
            println!("Listing all");
            process_list_subcommands(conn, &ListSubcommands::Tasks {})?;
            process_list_subcommands(conn, &ListSubcommands::Events {})?;
            // process_list_subcommands(conn, &ListSubcommands::Workflows {})?;
            process_list_subcommands(conn, &ListSubcommands::Engines {})
        }
    }
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
    pretty_table.add_row(Row::new(
        fields
            .iter()
            .map(|field| Cell::new(field).style_spec("Fg"))
            .collect(),
    ));

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
                    if s.len() > PRETTY_TABLE_MAX_CELL_LEN {
                        s.truncate(PRETTY_TABLE_MAX_CELL_LEN);
                        format!("{}...", s)
                    } else {
                        s
                    }
                })
                .collect(),
        );
    }

    pretty_table.printstd();
    Ok(())
}

fn main() -> Result<(), AnyError> {
    cli();
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
