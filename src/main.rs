use anyhow::Error as AnyError;
use bincode::{deserialize, serialize};
use chrono::prelude::*;
use ctrlc::set_handler;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use rayon::ThreadPoolBuilder;
use redis::{Commands, FromRedisValue, RedisResult};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fmt};
use std::{str, thread};

#[cfg(test)]
mod tests;

struct Event {
    uid: u32,
    name: String,
    description: String,
    date: String,
    time: String,
    content: String,
}

enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Task {
    uid: u32,
    name: String,
    description: String,
    date: String,
    time: String,
    file: String,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tname: {}", self.name)?;
        writeln!(f, "\tdescription: {}", self.description)?;
        writeln!(f, "\tdate: {}", self.date)?;
        writeln!(f, "\ttime: {}", self.time)?;
        writeln!(f, "\tfile: {}", self.file)?;
        Ok(())
    }
}

impl Event {
    fn execute(&self) {
        println!("Event: {}", self.name);
    }
}

// replace with a database
struct EventStore {
    events: Vec<Event>,
    tasks: Vec<Task>,
}

fn main() -> Result<(), AnyError> {
    println!("Hello, world!");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let thread_pool_result = ThreadPoolBuilder::new().num_threads(4).build();
    if let Err(e) = thread_pool_result {
        println!("Failed to create thread pool {}", e);
        println!("exiting...");
        std::process::exit(1);
    }
    let thread_pool = thread_pool_result.unwrap();

    if let Err(e) = create_initial_tables() {
        println!("Failed to create initial tables {}", e);
        println!("exiting...");
        std::process::exit(1);
    }
    println!("Created initial postgres tables");

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

    // serialization is needed to store the task in redis
    let serialized_task = serialize(&mock_task).unwrap();

    redis_con.rpush("test", serialized_task)?;

    while running.load(Ordering::SeqCst) {
        let task: Option<redis::Value> = redis_con.lpop("test", Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                // If the program exists, then thread_pool will be dropped and all threads will be stopped
                // which means that threads will not be able to complete their current task
                thread_pool.spawn(move || {
                    let task: Task = deserialize(&popped_value.as_bytes()).unwrap();
                    println!("Task: {}", task);
                    if let Err(e) = task_executor(task) {
                        println!("Failed to execute task {}", e);
                    };
                });
            }
            None => {
                println!("No task");
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    println!("\nCtrl+C signal detected. Exiting...");
    Ok(())
}

fn create_initial_tables() -> Result<(), Error> {
    let mut postgres_client = get_postgres_client()?;

    //using postgres, create a table to store the state of workflow engine tasks
    postgres_client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            uid             SERIAL PRIMARY KEY,
            name            VARCHAR NOT NULL,
            description     VARCHAR NOT NULL,
            file            VARCHAR NOT NULL,
            status          VARCHAR NOT NULL,
            created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            deleted_at      TIMESTAMP,
            completed_at    TIMESTAMP
        )
        ",
    )
}

fn get_postgres_client() -> Result<Client, Error> {
    dotenv().ok();
    let postgres_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not set");
    let client = Client::connect(
        format!(
            "host=localhost user=postgres password={}",
            postgres_password
        )
        .as_str(),
        NoTls,
    )?;

    Ok(client)
}

fn get_redis_con() -> RedisResult<redis::Connection> {
    dotenv().ok();
    let client = redis::Client::open(env::var("REDIS_URL").expect("Redis url not set"))?;
    let con = client.get_connection()?;
    Ok(con)
}

fn task_executor(task: Task) -> Result<(), AnyError> {
    println!("Task Executor");

    let postgres_result = get_postgres_client();
    let mut postgres_client = postgres_result?;

    let task_uid = postgres_client.execute(
        "INSERT INTO tasks (name, description, file, status) VALUES ($1, $2, $3, $4)",
        &[
            &task.name,
            &task.description,
            &task.file,
            &TaskStatus::Running.to_string(),
        ],
    )?;

    // thread::sleep(Duration::from_millis(5000));
    let output = Command::new("sh")
        .arg(task.file)
        .output()
        .expect("failed to execute process");

    postgres_client.execute(
        "UPDATE tasks SET status = $1 , updated_at = NOW(), completed_at = NOW() WHERE uid = $2",
        &[&TaskStatus::Completed.to_string(), &(task_uid as i32)],
    )?;

    println!("status: {}", output.status);
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
