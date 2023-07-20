use anyhow::Error as AnyError;
use bincode::{deserialize, serialize};
use ctrlc::set_handler;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use rayon::ThreadPoolBuilder;
use redis::{Commands as RedisCommand, FromRedisValue, RedisResult};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::os::unix::process;
use std::path::Path;
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
    Retrying,
}

impl Display for EventStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventStatus::Created => write!(f, "Created"),
            EventStatus::Succeeded => write!(f, "Succeeded"),
            EventStatus::Retrying => write!(f, "Retrying"),
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
    pub event_uid: i32,
    pub name: String,
    pub description: String,
    pub status: String,
    pub path: String,
    pub on_failure: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: String,
    pub completed_at: String,
}

impl fmt::Display for EngineTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tevent_uid: {}", self.event_uid)?;
        writeln!(f, "\tname: {}", self.name)?;
        writeln!(f, "\tdescription: {}", self.description)?;
        writeln!(f, "\tstatus: {}", self.status)?;
        writeln!(f, "\tpath: {}", self.path)?;
        writeln!(f, "\ton_failure: {}", self.on_failure)?;
        writeln!(f, "\tcreated_at: {}", self.created_at)?;
        writeln!(f, "\tupdated_at: {}", self.updated_at)?;
        writeln!(f, "\tdeleted_at: {}", self.deleted_at)?;
        writeln!(f, "\tcompleted_at: {}", self.completed_at)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct LightTask {
    pub uid: i32,
    pub path: String,
    pub on_failure: Option<String>,
}

impl Display for LightTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tpath: {}", self.path)?;
        writeln!(
            f,
            "\ton_failure: {}",
            self.on_failure.as_ref().unwrap_or(&"None".to_string())
        )?;
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

fn run_process<F>(process_name: &str, process_fn: F) -> Result<(), AnyError>
where
    F: FnOnce(Arc<AtomicBool>) -> Result<(), AnyError>,
{
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(e) = process_fn(running) {
        eprintln!("Failed to start {} process: {}", process_name, e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("{} process stopped correctly", process_name);

    Ok(())
}

pub fn run_task_process() -> Result<(), AnyError> {
    run_process("Task", queue_processor)
}

pub fn run_event_process() -> Result<(), AnyError> {
    run_process("Event", poll_events)
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

fn queue_processor(running: Arc<AtomicBool>) -> Result<(), AnyError> {
    let thread_pool_result = ThreadPoolBuilder::new().num_threads(4).build();
    if let Err(e) = thread_pool_result {
        eprintln!("Failed to create thread pool {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let thread_pool = thread_pool_result.unwrap();

    let redis_result = create_redis_connection();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut redis_con = redis_result.unwrap();

    let mut postgres_client = create_postgres_client()?;
    postgres_client.execute("
    INSERT INTO engine_status (status, started_at) VALUES ($1, NOW()) ON CONFLICT (id) DO UPDATE SET status = $1, started_at = NOW() WHERE engine_status.id = 1;
    ", &[&EngineStatus::Running.to_string()])?;

    while running.load(Ordering::SeqCst) {
        let task: Option<redis::Value> = redis_con.lpop(utils::QUEUE_NAME, Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                // If the program exists, then thread_pool will be dropped and all threads will be stopped
                // which means that threads will not be able to complete their current task
                thread_pool.spawn(move || {
                    let task: LightTask = deserialize(popped_value.as_bytes()).unwrap();
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

    if !running.load(Ordering::SeqCst) {
        println!("\nCtrl+C signal detected. Exiting...");
    }

    postgres_client.execute(
        "
        UPDATE engine_status SET status = $1, stopped_at = NOW() WHERE ID = 1",
        &[&EngineStatus::Stopped.to_string()],
    )?;
    Ok(())
}

pub fn initialize_tables() -> Result<(), Error> {
    // let redis_result = create_redis_connection();
    // if let Err(e) = redis_result {
    //     eprintln!("Failed to connect to redis {}", e);
    //     eprintln!("exiting...");
    //     std::process::exit(1);
    // }
    // let mut redis_con = redis_result.unwrap();

    // // delete redis queue
    // redis_con.del(utils::QUEUE_NAME)?;

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
    )?;
    println!("Created initial postgres tables");
    Ok(())
}

fn poll_events(running: Arc<AtomicBool>) -> Result<(), AnyError> {
    let mut postgres_client = create_postgres_client()?;

    let mut event_uids: Vec<i32> = Vec::new();

    while running.load(Ordering::SeqCst) {
        let events = postgres_client.query(
            "SELECT uid, name, description, trigger, status FROM events WHERE status != $1",
            &[&EventStatus::Succeeded.to_string()],
        )?;

        for event in events {
            let event_uid: i32 = event.get("uid");

            let event_name: String = event.get("name");
            let event_description: String = event.get("description");
            let event_trigger: String = event.get("trigger");
            let event_status: String = event.get("status");

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
    if !running.load(Ordering::SeqCst) {
        println!("\nCtrl+C signal detected. Exiting...");
    }
    Ok(())
}

fn execute_task(task: LightTask) -> Result<(), AnyError> {
    println!("Task Executor");

    let postgres_result = create_postgres_client();
    let mut postgres_client = postgres_result?;

    let path_basename = Path::new(&task.path).file_name().unwrap();
    let path_dirname = Path::new(&task.path).parent().unwrap();

    let output = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .output()
        .expect("failed to execute process");

    if output.status.code().unwrap() == 0 {
        postgres_client.execute(
            "UPDATE tasks SET status = $1 , updated_at = NOW(), completed_at = NOW() WHERE uid = $2",
            &[&TaskStatus::Completed.to_string(), &task.uid],
        )?;
        // TODO on failure
    } else {
        postgres_client.execute(
            "UPDATE tasks SET status = $1 , updated_at = NOW(), completed_at = NOW() WHERE uid = $2",
            &[&TaskStatus::Failed.to_string(), &task.uid],
        )?;
    }

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

    let path_basename = Path::new(&event.trigger).file_name().unwrap(); // TODO
    let path_dirname = Path::new(&event.trigger).parent().unwrap(); // TODO
                                                                    // thread::sleep(Duration::from_millis(5000));
    let output = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .output()
        .expect("failed to execute process");

    // if shell command return 0, then the event was triggered successfully
    if output.status.code().unwrap() == 0 {
        postgres_client.execute(
            "UPDATE events SET status = $1 , triggered_at = NOW() WHERE uid = $2",
            &[&EventStatus::Succeeded.to_string(), &event.uid],
        )?;
        // push tasks uid to queue
        let event_tasks = postgres_client.query(
            "SELECT uid, path, on_failure FROM tasks WHERE event_uid = $1",
            &[&event.uid],
        )?;
        let light_tasks: Vec<LightTask> = event_tasks
            .iter()
            .map(|row| {
                let uid: i32 = row.get("uid");
                let path: String = row.get("path");
                let on_failure: Option<String> = row.get("on_failure");
                LightTask {
                    uid,
                    path,
                    on_failure,
                }
            })
            .collect::<Vec<LightTask>>();
        let _ = push_tasks_to_queue(light_tasks);
    } else {
        postgres_client.execute(
            "UPDATE events SET status = $1 , triggered_at = NOW() WHERE uid = $2",
            &[&EventStatus::Retrying.to_string(), &event.uid],
        )?;
    };

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
