use anyhow::Error as AnyError;
use bincode::{deserialize, serialize};
use ctrlc::set_handler;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use rayon::ThreadPoolBuilder;
use redis::{Commands as RedisCommand, FromRedisValue, RedisResult};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::process::Command as ShellCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, fmt};
use std::{str, thread};

use std::fs::File;

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

enum EngineStatus {
    Running,
    Stopped,
}

impl Display for EngineStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EngineStatus::Running => write!(f, "Running"),
            EngineStatus::Stopped => write!(f, "Stopped"),
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

pub fn start_handler() -> Result<(), AnyError> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // spawn a new process to run the engine
    let engine_running = running.clone();

    if let Err(e) = start_engine(engine_running) {
        eprintln!("Failed to start engine, {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("Engine stopped correctly");

    Ok(())
}

pub fn stop_handler() -> Result<(), AnyError> {
    let mut postgres_client = get_postgres_client()?;
    postgres_client.execute(
        "
    UPDATE engine_status SET stop_signal = true WHERE ID = 1;
    ",
        &[],
    )?;
    Ok(())
}

fn start_engine(running: Arc<AtomicBool>) -> Result<(), AnyError> {
    // // Open the file in write-only mode
    // let file = File::create("output.txt").expect("Failed to create file");

    // // Obtain the raw file descriptor
    // let file_descriptor = file.as_raw_fd();

    // // Create a writeable handle from the file descriptor
    // let mut writeable = unsafe { File::from_raw_fd(file_descriptor) };

    let thread_pool_result = ThreadPoolBuilder::new().num_threads(4).build();
    if let Err(e) = thread_pool_result {
        eprintln!("Failed to create thread pool {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let thread_pool = thread_pool_result.unwrap();
    if let Err(e) = create_initial_tables() {
        eprintln!("Failed to create initial tables {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("Created initial postgres tables");
    let redis_result = get_redis_con();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        eprintln!("exiting...");
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
    let mut postgres_client = get_postgres_client()?;
    postgres_client.execute("
    INSERT INTO engine_status (status, started_at) VALUES ($1, NOW()) ON CONFLICT (id) DO UPDATE SET status = $1, started_at = NOW() WHERE engine_status.id = 1;
    ", &[&EngineStatus::Running.to_string()])?;

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
                        eprintln!("Failed to execute task {}", e);
                    };
                });
            }
            None => {
                println!("No task");
            }
        }

        // check is the engine has received a stop signal
        let received_stop_signal_result = postgres_client.query(
            "SELECT stop_signal FROM engine_status WHERE stop_signal = true",
            &[],
        );
        match received_stop_signal_result {
            Ok(rows) => {
                if !rows.is_empty() {
                    println!("Received stop signal");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to query engine status {}", e);
                eprintln!("exiting...");
                std::process::exit(1);
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
    println!("\nCtrl+C signal detected. Exiting...");

    postgres_client.execute(
        "
        UPDATE engine_status SET status = $1, stopped_at = NOW() WHERE ID = 1",
        &[&EngineStatus::Stopped.to_string()],
    )?;
    Ok(())
}

fn create_initial_tables() -> Result<(), Error> {
    let mut postgres_client = get_postgres_client()?;

    //using postgres, create a table to store the state of workflow engine tasks
    // TODO: remove constraint on engine_status table after implementing multiple engine instances
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
        );

        
        DROP TABLE IF EXISTS engine_status;
        CREATE TABLE IF NOT EXISTS engine_status (
            id              SERIAL PRIMARY KEY,
            status          VARCHAR NOT NULL,
            stop_signal     BOOLEAN NOT NULL DEFAULT false,
            started_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            stopped_at      TIMESTAMP NOT NULL DEFAULT NOW()
        );

        ALTER TABLE engine_status ADD CONSTRAINT engine_status_unique CHECK (id = 1);
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
    let output = ShellCommand::new("sh")
        .arg(task.file)
        .output()
        .expect("failed to execute process");

    postgres_client.execute(
        "UPDATE tasks SET status = $1 , updated_at = NOW(), completed_at = NOW() WHERE uid = $2",
        &[&TaskStatus::Completed.to_string(), &(task_uid as i32)],
    )?;

    println!("status: {}", output.status);
    // write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
