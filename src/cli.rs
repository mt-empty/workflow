// use anyhow::{Error as AnyError, Ok, Result};
use clap::{Error, Parser, Subcommand};
// use std::fs::File;
// use std::io::{BufRead, BufReader, Cursor, Read};
// use std::path::{Path, PathBuf};
// mod utils;
// use utils::get_redis_con;

const PAGES_BASE_DIR: &str = "";

#[derive(Parser)]
#[clap(arg_required_else_help(true))]
#[clap(about = "A tool to manipulate  workflow tasks", author, version)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    #[clap(about = "Adds a task")]
    Add(AddCommand),
    #[clap(about = "Shows the status of a task")]
    Show(ShowCommand),
    #[clap(about = "Stops the task")]
    Stop(StopCommand),
    #[clap(about = "Continues the task")]
    Continue(ContinueCommand),
    #[clap(about = "Aborts the task")]
    Abort(AbortCommand),
    #[clap(about = "Lists all tasks")]
    List(ListCommand),
}

#[derive(Clap, Debug)]
struct AddCommand {
    #[clap(index = 1, required = true)]
    file: String,
}

#[derive(Clap, Debug)]
struct ShowCommand {
    #[clap(index = 1, required = true)]
    task_name: String,
}

#[derive(Clap, Debug)]
struct StopCommand {
    #[clap(index = 1, required = true)]
    task_name: String,
}

#[derive(Clap, Debug)]
struct ContinueCommand {
    #[clap(index = 1, required = true)]
    task_name: String,
}

#[derive(Clap, Debug)]
struct AbortCommand {
    #[clap(index = 1, required = true)]
    task_name: String,
}

#[derive(Clap, Debug)]
struct ListCommand {}

pub fn create_cli() -> MyCli {
    MyCli::parse()
}

fn cli() {
    let cli = cli::create_cli();

    match cli.subcmd {
        SubCommand::Add(add_cmd) => {
            println!("Adding file: {}", add_cmd.file);
            if let Err(e) = add(add_cmd.file) {
                println!("Failed to add file {}", e);
                std::process::exit(1);
            }
            // Add your logic for the 'add' subcommand here
        }
        SubCommand::Show(check_cmd) => {
            println!("Showing task : {}", show_cmd.task_name);
            // Add your logic for the 'check' subcommand here
        }
        SubCommand::Stop(stop_cmd) => {
            println!("Stopping task : {}", stop_cmd.task_name);
            // Add your logic for the 'check' subcommand here
        }
        SubCommand::Continue(continue_cmd) => {
            println!("Continuing task : {}", continue_cmd.task_name);
            // Add your logic for the 'check' subcommand here
        }
        SubCommand::Abort(abort_cmd) => {
            println!("Deleting task : {}", abort_cmd.task_name);
            // Add your logic for the 'check' subcommand here
        }
        SubCommand::List(_) => {
            println!("Showing status");
            // Add your logic for the 'status' subcommand here
        }
    }
}

fn add(file_path: String) -> Result<(), Error> {
    // open yaml file
    let file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_yaml::from_str(&contents)?;
    println!("{:?}", config);

    let redis_result = get_redis_con();
    if let Err(e) = redis_result {
        println!("Failed to connect to redis {}", e);
        println!("exiting...");
        std::process::exit(1);
    }
    let mut redis_con = redis_result.unwrap();
    let mock_task = Task {
        uid: 1,
        name: "name".to_string(),
        description: "description".to_string(),
        date: "date".to_string(),
        time: "time".to_string(),
        file: "./tests/tasks/create_foo.sh".to_string(),
    };
    let serialized_task = serialize(&mock_task).unwrap();
    redis_con.rpush("test", serialized_task)?;

    Ok(())
}
