use crate::models::{Engine, EngineStatus, LightTask, TaskStatus};
use crate::utils::{self, create_redis_connection, establish_pg_connection};
use anyhow::Error as AnyError;
use bincode::deserialize;
use diesel::prelude::*;
use rayon::ThreadPoolBuilder;
use redis::{Commands as RedisCommand, FromRedisValue};
use std::path::Path;
use std::process::Command as ShellCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{str, thread};

const THREAD_COUNT: usize = 4;

pub fn queue_processor(running: Arc<AtomicBool>, engine_uid: i32) -> Result<(), AnyError> {
    let thread_pool = ThreadPoolBuilder::new().num_threads(THREAD_COUNT).build()?;
    let pg_conn = &mut establish_pg_connection();
    let mut redis_con = create_redis_connection()?;

    use crate::schema::engines::dsl::*;
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
                        println!("Failed to execute task {}", e);
                    };
                });
            }
            None => {
                println!("No task to process");
            }
        }

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
    diesel::update(engines.find(engine_uid))
        .set((
            stopped_at.eq(diesel::dsl::now),
            status.eq(EngineStatus::Stopped.to_string()),
        ))
        .get_result::<Engine>(pg_conn)?;

    Ok(())
}

fn execute_task(task: LightTask) -> Result<(), AnyError> {
    println!("Task Executor");

    use crate::schema::tasks::dsl::*;
    let mut conn = establish_pg_connection();
    // todo , update task status to running

    diesel::update(tasks.find(task.uid))
        .set((
            status.eq(TaskStatus::Running.to_string()),
            updated_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)?;

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
