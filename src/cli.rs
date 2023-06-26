use anyhow::{Error as AnyError, Ok, Result};
use clap::{Parser, Subcommand};
use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_yaml::from_str;
use std::{
    fs::{read_to_string, File},
    io::Read,
};

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
    /// Adds workflow to the queue
    Add {
        file_path: String,
    },
    // Shows the status of a task
    Show {
        task_name: String,
    },
    // Stops the task
    Stop {
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
        Commands::Add { file_path } => {
            println!("Adding file: {}", file_path);
            if let Err(e) = add(file_path.to_string()) {
                println!("Failed to add file {}", e);
                std::process::exit(1);
            }
            // Add your logic for the 'add' Commands here
        }
        Commands::Show { task_name } => {
            println!("Showing task: {}", task_name);
            // Add your logic for the 'show' Commands here
        }
        Commands::Stop { task_name } => {
            println!("Stopping task: {}", task_name);
            // Add your logic for the 'stop' Commands here
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

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    name: String,
    description: String,
    data_sources: Vec<String>,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub name: String,
    pub description: String,
    pub events: Events,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Events {
    pub file_exists: FileExists,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileExists {
    pub path: String,
    pub success: Success,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Success {
    pub action: Action,
    pub failure: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub path: String,
}

fn add(file_path: String) -> Result<(), AnyError> {
    // open yaml file
    // let mut map = BTreeMap::new();
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let config: Config = serde_yaml::from_reader(file)?;
    println!("{:?}", config);

    // let redis_result = get_redis_con();
    // if let Err(e) = redis_result {
    //     println!("Failed to connect to redis {}", e);
    //     println!("exiting...");
    //     std::process::exit(1);
    // }
    // let mut redis_con = redis_result.unwrap();
    // let mock_task = Task {
    //     uid: 1,
    //     name: "name".to_string(),
    //     description: "description".to_string(),
    //     date: "date".to_string(),
    //     time: "time".to_string(),
    //     file: "./tests/tasks/create_foo.sh".to_string(),
    // };
    // let serialized_task = serialize(&mock_task).unwrap();
    // redis_con.rpush("test", serialized_task)?;

    Ok(())
}
