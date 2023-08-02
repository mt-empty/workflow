use anyhow::{Error as AnyError, Result};
use bincode::serialize;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use redis::{Commands, RedisResult};
use std::env;

use crate::models::{LightTask, NewEvent, NewTask};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::info;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub const QUEUE_NAME: &str = "tasks";

pub fn create_redis_connection() -> RedisResult<redis::Connection> {
    dotenv().ok();
    let client = redis::Client::open(env::var("REDIS_URL").expect("Redis url not set"))?;
    let con = client.get_connection()?;
    Ok(con)
}

pub fn establish_pg_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)) // TODO: handle this error
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

// TODO, insert workflow into db, instead of the two functions bellow
pub fn insert_event_into_db(conn: &mut PgConnection, new_event: NewEvent) -> Result<i32, AnyError> {
    use crate::schema::events::dsl::*;
    let event_uid = diesel::insert_into(events)
        .values(&new_event)
        .returning(uid)
        .get_result::<i32>(conn)?;
    Ok(event_uid)
}

pub fn insert_event_tasks_into_db(
    conn: &mut PgConnection,
    new_tasks: Vec<NewTask>,
) -> Result<(), AnyError> {
    for new_task in new_tasks {
        diesel::insert_into(crate::schema::tasks::table)
            .values(new_task)
            .execute(conn)?; // TODO: check if this is the correct way to do it
    }

    Ok(())
}

pub fn run_migrations() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    info!("Running Database migrations...");
    let mut conn = establish_pg_connection();
    conn.run_pending_migrations(MIGRATIONS)?;
    info!("Database migrations complete.");
    Ok(())
}
