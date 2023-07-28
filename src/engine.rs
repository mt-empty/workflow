use crate::models::Engine;
use crate::schema::*;
use crate::utils::{self, create_redis_connection, establish_pg_connection, push_tasks_to_queue};
use crate::{models, schema};
use anyhow::Error as AnyError;
use bincode::{deserialize, serialize};
use ctrlc::set_handler;
use diesel::sql_types::*;
use diesel::PgConnection;
use dotenv::dotenv;
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

#[derive(Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::tasks)]
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

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::events)]
pub struct EngineEvent {
    pub uid: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub trigger: String,
    pub status: String,
}

impl fmt::Display for EngineEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "\tuid: {}", self.uid)?;
        writeln!(f, "\tname: {:?}", self.name)?;
        writeln!(f, "\tdescription: {:?}", self.description)?;
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
// set engine_uid to 1 by default
pub fn handle_stop(engine_uid: i32) -> Result<(), AnyError> {
    diesel::update(schema::engines::dsl::engines.find(engine_uid))
        .set(schema::engines::stop_signal.eq(true))
        .execute(&mut establish_pg_connection())?;
    Ok(())
}

use diesel::prelude::*;

pub fn create_engine(
    conn: &mut PgConnection,
    name: &str,
    ip_address: &str,
) -> Result<i32, diesel::result::Error> {
    use crate::schema::engines::table as engines;
    use crate::schema::engines::uid as engine_uid;

    let new_engine = models::NewEngine { name, ip_address };

    //insert and return uid
    diesel::insert_into(engines)
        .values(&new_engine)
        .returning(engine_uid)
        .get_result::<i32>(conn)
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

    let pg_conn = &mut establish_pg_connection();
    let engine_uid = create_engine(pg_conn, "first", "0.0.0.0")?;

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

        use crate::schema::engines::dsl::*;
        // rust couldn't infer the type of received_stop_signal_result
        let received_stop_signal_result: Result<Option<bool>, _> = engines
            .find(engine_uid)
            .select(stop_signal)
            .first(pg_conn)
            .optional();
        match received_stop_signal_result {
            Ok(Some(signal_on)) => {
                if signal_on {
                    println!("Received stop signal");
                    break;
                }
            }
            Ok(None) => {
                println!("No stop signal");
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
    let updated_results = diesel::update(schema::engines::dsl::engines.find(engine_uid))
        .set((
            schema::engines::stopped_at.eq(diesel::dsl::now),
            schema::engines::status.eq(EngineStatus::Stopped.to_string()),
        ))
        .get_result::<Engine>(pg_conn)?;

    Ok(())
}

fn poll_events(running: Arc<AtomicBool>) -> Result<(), AnyError> {
    let mut event_uids: Vec<i32> = Vec::new();
    let mut conn = establish_pg_connection();
    while running.load(Ordering::SeqCst) {
        let events: Vec<EngineEvent> = schema::events::dsl::events
            .select(EngineEvent::as_select())
            .filter(schema::events::status.ne(EventStatus::Succeeded.to_string()))
            .load(&mut conn)?;

        for event in events {
            println!("Event: {}", event);
            // async execute_event
            let _ = execute_event(event);
        }

        if event_uids.is_empty() {
            println!("No events to process");
            thread::sleep(Duration::from_millis(2000));
        }

        use crate::schema::engines::dsl::*;
        let received_stop_signal_result: Result<Option<bool>, _> = engines
            .find(1)
            .select(stop_signal)
            .first(&mut conn)
            .optional();
        match received_stop_signal_result {
            Ok(Some(signal_on)) => {
                if signal_on {
                    println!("Received stop signal");
                    break;
                }
            }
            Ok(None) => {
                println!("No stop signal");
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

    use crate::schema::tasks::dsl::*;
    let mut conn = establish_pg_connection();
    // todo , update task status to running
    let path_basename = match Path::new(&task.path).file_name() {
        Some(basename) => basename,
        None => return Err(AnyError::msg("Failed to get path basename")),
    };
    let path_dirname = Path::new(&task.path).parent().unwrap();

    let output = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .output()
        .expect("failed to execute process");

    if output.status.code().unwrap() == 0 {
        diesel::update(tasks.find(task.uid))
            .set((
                status.eq(TaskStatus::Completed.to_string()),
                updated_at.eq(diesel::dsl::now),
                completed_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)?;

        // TODO run on failure
    } else {
        diesel::update(tasks.find(task.uid))
            .set((
                status.eq(TaskStatus::Failed.to_string()),
                updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)?;
    }

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}

fn execute_event(event: EngineEvent) -> Result<(), AnyError> {
    println!("Event Executor");

    let mut conn = establish_pg_connection();

    let path_basename = match Path::new(&event.trigger).file_name() {
        Some(basename) => basename,
        None => return Err(AnyError::msg("Failed to get path basename")),
    };

    let path_dirname = Path::new(&event.trigger).parent().unwrap();
    // thread::sleep(Duration::from_millis(5000));
    let output = ShellCommand::new("bash")
        .arg(path_basename)
        .current_dir(path_dirname)
        .output()
        .expect("failed to execute process");

    // if shell command return 0, then the event was triggered successfully
    if output.status.code().unwrap() == 0 {
        {
            use crate::schema::events::dsl::*;
            diesel::update(events.find(event.uid))
                .set(status.eq(EventStatus::Succeeded.to_string()))
                .execute(&mut conn)?;
        }
        use crate::schema::tasks::dsl::*;

        let light_tasks: Vec<LightTask> = tasks
            .select(LightTask::as_select())
            .filter(event_uid.eq(event.uid))
            .load(&mut conn)?;
        let _ = push_tasks_to_queue(light_tasks);
    } else {
        use crate::schema::events::dsl::*;
        diesel::update(events.find(event.uid))
            .set((
                status.eq(EventStatus::Retrying.to_string()),
                triggered_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)?;
    };

    println!("status: {}", output.status);
    // TODO: write to disk
    println!("stdout: {}", str::from_utf8(&output.stdout)?);
    println!("stderr: {}", str::from_utf8(&output.stderr)?);
    Ok(())
}
