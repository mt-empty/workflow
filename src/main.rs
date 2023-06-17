use bincode::{deserialize, serialize};
use ctrlc;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use redis::{Commands, Connection, FromRedisValue, RedisResult, ToRedisArgs};
use serde::{Deserialize, Serialize};
use std::env;
use std::num::NonZeroUsize;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{str, thread};
use rayon::prelude::*;

struct Event {
    uid: u32,
    name: String,
    description: String,
    date: String,
    time: String,
    file: String,
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

fn main() -> RedisResult<()> {
    println!("Hello, world!");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let redis_result = get_redis_con();
    let postgres_result = postgres();

    let mut redis_con = redis_result.unwrap();

    let full_task = Task {
        uid: 1,
        name: "test".to_string(),
        description: "test".to_string(),
        date: "test".to_string(),
        time: "test".to_string(),
        file: "./testcases/create_foo.sh".to_string(),
    };

    let serialized_task = serialize(&full_task).unwrap();

    redis_con.rpush("test", serialized_task)?;

    while running.load(Ordering::SeqCst) {
        let task: Option<redis::Value> = redis_con.lpop("test", Default::default())?;
        match task {
            Some(value) => {
                let popped_value: String = FromRedisValue::from_redis_value(&value)?;
                println!("Task: {}", popped_value);
                // asynchronously execute task using task_executor, this should create a new thread
                let thread_handle = thread::spawn(move || {
                    let task: Task = deserialize(&popped_value.as_bytes()).unwrap();
                    task_executor(task);
                });
            }
            None => {
                println!("No task ");
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    println!("Ctrl+C signal detected. Exiting...");
    thread_handle
        .join()
        .expect("Failed to join the new thread.");

    Ok(())

    // let event: Event = Event {
    //     uid: 1,
    //     name: "test".to_string(),
    //     description: "test".to_string(),
    //     date: "test".to_string(),
    //     time: "test".to_string(),
    //     file: "".to_string(),
    // };

    // let action: Action = Action {
    //     uid: 1,
    //     name: "test".to_string(),
    //     description: "test".to_string(),
    //     date: "test".to_string(),
    //     time: "test".to_string(),
    //     file: "test".to_string(),
    // };

    // let mut event_store = EventStore {
    //     events : Vec::new(),
    //     actions: Vec::new()
    // };

    // event_store.events.push(event);
    // event_store.actions.push(action);

    // engine(event_store);
}

fn postgres() -> Result<Client, Error> {
    dotenv().ok();
    let postgres_password = env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD not set");
    let mut client = Client::connect(
        format!(
            "host=localhost user=postgres password={}",
            postgres_password
        )
        .as_str(),
        NoTls,
    )?;
    println!("Connected to postgres");

    //using postgres, create a table to store the state of workflow engine tasks
    client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS events (
            uid             SERIAL PRIMARY KEY,
            name            VARCHAR NOT NULL,
            description     VARCHAR NOT NULL,
            date            VARCHAR NOT NULL,
            time            VARCHAR NOT NULL,
            file            VARCHAR NOT NULL,
            status          VARCHAR NOT NULL,
            created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at      TIMESTAMP NOT NULL DEFAULT NOW(),
            deleted_at      TIMESTAMP,
            completed_at    TIMESTAMP
        )
        ",
    )?;

    Ok(client)
}

fn get_redis_con() -> RedisResult<redis::Connection> {
    // connect to redis
    let client = redis::Client::open("redis://172.17.0.2/")?;
    let con = client.get_connection()?;
    Ok(con)
}

fn task_executor(task: Task) {
    println!("Task Executor");
    let output = Command::new("sh")
        .arg(task.file)
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
    println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
}

// fn executor(file_path: String) {
//     println!("executing  file: {}", file_path);
//     let output = Command::new("ls")
//         .arg("a")
//         .arg(file_path)
//         .output()
//         .expect("failed to execute process");

//     println!("status: {}", output.status);
//     println!("stdout: {}", str::from_utf8(&output.stdout).unwrap());
//     println!("stderr: {}", str::from_utf8(&output.stderr).unwrap());
// }

// fn engine(event_store: EventStore) {
//     println!("Engine");

//     loop {
//         for event in event_store.events {
//             println!("Event: {}", event.name);

//             if executor(event) != None {
//                 println!("Successfully executed event: {}", event.name);
//             }
//         }
//         // sleep for 1 second
//         std::thread::sleep(std::time::Duration::from_secs(1));
//     }
// }
