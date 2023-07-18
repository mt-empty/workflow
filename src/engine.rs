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
use std::thread::sleep;
use std::time::Duration;
use std::{env, fmt, str, thread};

use crate::utils::{self, create_postgres_client, create_redis_connection, push_tasks_to_queue};
use std::fs::File;

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

pub enum EventStatus {
    Created,
    Succeeded,
    Failed,
}

impl Display for EventStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventStatus::Created => write!(f, "Created"),
            EventStatus::Succeeded => write!(f, "Succeeded"),
            EventStatus::Failed => write!(f, "Failed"),
        }
    }
}

pub enum EngineStatus {
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
pub struct EngineTask {
    pub uid: i32,
    pub name: String,
    pub description: String,
    pub date: String,
    pub time: String,
    pub path: String,
}

impl fmt::Display for EngineTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tname: {}", self.name)?;
        writeln!(f, "\tdescription: {}", self.description)?;
        writeln!(f, "\tdate: {}", self.date)?;
        writeln!(f, "\ttime: {}", self.time)?;
        writeln!(f, "\tpath: {}", self.path)?;
        Ok(())
    }
}

pub struct EngineEvent {
    pub uid: i32,
    pub name: String,
    pub description: String,
    pub trigger: String,
    pub status: String,
}

impl fmt::Display for EngineEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tname: {}", self.name)?;
        writeln!(f, "\tdescription: {}", self.description)?;
        writeln!(f, "\ttrigger: {}", self.trigger)?;
        Ok(())
    }
}

pub fn handle_start() -> Result<(), AnyError> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let thread_pool_result = ThreadPoolBuilder::new().num_threads(2).build();
    let thread_pool = thread_pool_result.unwrap();
    if let Err(e) = initialize_tables() {
        eprintln!("Failed to create initial tables {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    // spawn poll_event
    let _ = poll_events();
    // thread_pool.spawn(move || {
    //     let _  poll_events();
    // });
    println!("Started event polling");

    if let Err(e) = workflow_engine(running) {
        eprintln!("Failed to start engine, {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("Engine stopped correctly");

    Ok(())
}

pub fn handle_stop() -> Result<(), AnyError> {
    let mut postgres_client = create_postgres_client()?;
    postgres_client.execute(
        "
    UPDATE engine_status SET stop_signal = true WHERE ID = 1;
    ",
        &[],
    )?;
    Ok(())
}

fn workflow_engine(running: Arc<AtomicBool>) -> Result<(), AnyError> {
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
    if let Err(e) = initialize_tables() {
        eprintln!("Failed to create initial tables {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("Created initial postgres tables");
    let redis_result = create_redis_connection();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut redis_con = redis_result.unwrap();
    // let mock_task = EngineTask {
    //     uid: 1,
    //     name: "name".to_string(),
    //     description: "description".to_string(),
    //     date: "date".to_string(),
    //     time: "time".to_string(),
    //     path: "./tests/tasks/create_foo.sh".to_string(),
    // };
    let mut postgres_client = create_postgres_client()?;
    postgres_client.execute("
    INSERT INTO engine_status (status, started_at) VALUES ($1, NOW()) ON CONFLICT (id) DO UPDATE SET status = $1, started_at = NOW() WHERE engine_status.id = 1;
    ", &[&EngineStatus::Running.to_string()])?;

    // let serialized_task = serialize(&mock_task).unwrap();
    // redis_con.rpush("test", serialized_task)?;
    while running.load(Ordering::SeqCst) {
        let task: Option<redis::Value> = redis_con.lpop(utils::QUEUE_NAME, Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                // If the program exists, then thread_pool will be dropped and all threads will be stopped
                // which means that threads will not be able to complete their current task
                thread_pool.spawn(move || {
                    let task: EngineTask = deserialize(popped_value.as_bytes()).unwrap();
                    println!("Task: {}", task);
                    if let Err(e) = execute_task(task) {
                        eprintln!("Failed to execute task {}", e);
                    };
                });
            }
            None => {
                println!("No task");
            }
        }

        // check if the engine has received a stop signal
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

        thread::sleep(Duration::from_millis(2000));
    }
    println!("\nCtrl+C signal detected. Exiting...");

    postgres_client.execute(
        "
        UPDATE engine_status SET status = $1, stopped_at = NOW() WHERE ID = 1",
        &[&EngineStatus::Stopped.to_string()],
    )?;
    Ok(())
}

fn initialize_tables() -> Result<(), Error> {
    let mut postgres_client = create_postgres_client()?;

    //using postgres, create a table to store the state of workflow engine tasks
    // TODO: remove constraint on engine_status table after implementing multiple engine instances
    postgres_client.batch_execute(
        "

        CREATE TABLE IF NOT EXISTS events (
            uid             SERIAL PRIMARY KEY,
            name            VARCHAR NOT NULL,
            description     VARCHAR NOT NULL,
            trigger         VARCHAR NOT NULL,
            status          VARCHAR NOT NULL,
            created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            triggered_at    TIMESTAMP,
            deleted_at      TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS tasks (
            uid             SERIAL PRIMARY KEY,
            event_uid       INTEGER NOT NULL,
            name            VARCHAR NOT NULL,
            description     VARCHAR NOT NULL,
            path            VARCHAR NOT NULL,
            status          VARCHAR NOT NULL,
            on_failure      VARCHAR,
            created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            deleted_at      TIMESTAMP,
            completed_at    TIMESTAMP,
            CONSTRAINT fk_event_uid
                FOREIGN KEY(event_uid)
                    REFERENCES events(uid) ON DELETE CASCADE ON UPDATE CASCADE
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

fn poll_events() -> Result<(), Error> {
    let mut postgres_client = create_postgres_client()?;

    let mut redis_con = create_redis_connection();

    let mut event_uids: Vec<i32> = Vec::new();

    loop {
        let events = postgres_client.query(
            "SELECT uid, name, description, trigger, status FROM events WHERE status != $1",
            &[&EventStatus::Succeeded.to_string()],
        )?;

        for event in events {
            let event_uid: i32 = event.get("uid");

            let event_name: String = event.get(1);
            let event_description: String = event.get(2);
            let event_trigger: String = event.get(3);
            let event_status: String = event.get(4);

            let event = EngineEvent {
                uid: event_uid,
                name: event_name,
                description: event_description,
                trigger: event_trigger,
                status: event_status,
            };
            println!("Event: {}", event);
            // async execute_event
            let _ = execute_event(event);
        }

        if event_uids.is_empty() {
            println!("No events to process");
            thread::sleep(Duration::from_millis(2000));
        }

        // check if the engine has received a stop signal
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

        event_uids.clear();
    }
    Ok(())
}

fn execute_task(task: EngineTask) -> Result<(), AnyError> {
    println!("Task Executor");

    let postgres_result = create_postgres_client();
    let mut postgres_client = postgres_result?;

    let task_uid = postgres_client.execute(
        "INSERT INTO tasks (name, description, path, status) VALUES ($1, $2, $3, $4)",
        &[
            &task.name,
            &task.description,
            &task.path,
            &TaskStatus::Running.to_string(),
        ],
    )?;

    // thread::sleep(Duration::from_millis(5000));
    let output = ShellCommand::new("sh")
        .arg(task.path)
        .output()
        .expect("failed to execute process");

    postgres_client.execute(
        "UPDATE tasks SET status = $1 , updated_at = NOW(), completed_at = NOW() WHERE uid = $2",
        &[&TaskStatus::Completed.to_string(), &(task_uid as i32)],
    )?;

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}

fn execute_event(event: EngineEvent) -> Result<(), AnyError> {
    println!("Event Executor");

    let postgres_result = create_postgres_client();
    let mut postgres_client = postgres_result?;

    // thread::sleep(Duration::from_millis(5000));
    let output = ShellCommand::new("sh")
        .arg(event.trigger)
        .output()
        .expect("failed to execute process");

    // if shell command return 0, then the event was triggered successfully
    if output.status.code().unwrap() == 0 {
        postgres_client.execute(
            "UPDATE events SET status = $1 , triggered_at = NOW() WHERE uid = $2",
            &[&EventStatus::Succeeded.to_string(), &(event.uid as i32)],
        )?;
        // push tasks uid to queue
        let event_tasks = postgres_client.query(
            "SELECT uid FROM tasks WHERE event_uid = $1",
            &[&(event.uid as i32)],
        )?;
        let converted_task_ids: Vec<i32> = event_tasks
            .iter()
            .map(|row| row.get(0))
            .collect::<Vec<i32>>();
        push_tasks_to_queue(&converted_task_ids);
    } else {
        postgres_client.execute(
            "UPDATE events SET status = $1 , triggered_at = NOW() WHERE uid = $2",
            &[&EventStatus::Failed.to_string(), &(event.uid as i32)],
        )?;
    };

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
