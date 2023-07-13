use anyhow::{Error as AnyError, Result};
use bincode::serialize;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use redis::{Commands, RedisResult};
use std::env;

use crate::engine::EngineTask;

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

pub fn push_task_to_queue(task: EngineTask) -> Result<(), AnyError> {
    let redis_result = create_redis_connection();
    if let Err(e) = redis_result {
        eprintln!("Failed to connect to redis {}", e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    let mut redis_con = redis_result.unwrap();

    let serialized_task = serialize(&task).unwrap();
    redis_con.rpush("test", serialized_task)?;
    Ok(())
}
