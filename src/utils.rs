use anyhow::{Error as AnyError, Result};
use bincode::serialize;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use redis::{Commands, RedisResult};
use std::env;

use crate::{
    engine::EngineTask,
    engine::{EventStatus, LightTask},
    parser::{Event, Task},
};

pub const QUEUE_NAME: &str = "tasks";

pub fn create_redis_connection() -> RedisResult<redis::Connection> {
    dotenv().ok();
    let client = redis::Client::open(env::var("REDIS_URL").expect("Redis url not set"))?;
    let con = client.get_connection()?;
    Ok(con)
}

pub fn create_postgres_client() -> Result<Client, Error> {
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
// This only pushes a light version of the task to the queue
pub fn push_tasks_to_queue(tasks: Vec<LightTask>) -> Result<(), AnyError> {
    let redis_result = create_redis_connection();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut redis_con = redis_result.unwrap();
    for light_task in tasks {
        let serialized_task: Vec<u8> = serialize(&light_task).unwrap();
        redis_con.rpush(QUEUE_NAME, serialized_task)?;
    }
    Ok(())
}

// TODO, insert workflow into bd, instead of the two functions bellow
pub fn insert_event_into_db(event: Event) -> Result<(), AnyError> {
    let client_result = create_postgres_client();
    if let Err(e) = client_result {
        eprintln!("Failed to connect to postgres {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut client = client_result.unwrap();

    let event_name = event.name;
    let event_description = event.description;
    let event_trigger = event.trigger;

    let result = client.query_one(
        "INSERT INTO events (name, description, trigger, status) VALUES ($1, $2, $3, $4) RETURNING uid",
        &[
            &event_name,
            &event_description,
            &event_trigger,
            &EventStatus::Created.to_string(),
        ],
    )?;
    println!("results: {:?}", result);
    let event_uid: i32 = result.get("uid");
    println!("uid: {:?}", event_uid);

    insert_event_tasks_into_db(event.tasks, event_uid)?;

    Ok(())
}

pub fn insert_event_tasks_into_db(tasks: Vec<Task>, event_uid: i32) -> Result<(), AnyError> {
    let client_result: std::result::Result<Client, Error> = create_postgres_client();
    if let Err(e) = client_result {
        eprintln!("Failed to connect to postgres {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut client = client_result.unwrap();

    for task in tasks {
        let task_name = task.name.unwrap_or("None".to_string());
        let task_description = task.description.unwrap_or("None".to_string());
        let task_path = task.path;
        let task_on_failure = task.on_failure;
        let task_status = EventStatus::Created.to_string();

        client.execute(
            "INSERT INTO tasks (event_uid, name, description, path, on_failure, status) VALUES ($1, $2, $3, $4, $5, $6)",
            &[&event_uid, &task_name, &task_description, &task_path, &task_on_failure, &task_status],
        )?;
    }

    Ok(())
}
