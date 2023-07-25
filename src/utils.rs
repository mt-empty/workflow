use anyhow::{Error as AnyError, Result};
use bincode::serialize;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use postgres::{Client, Error, NoTls};
use redis::{Commands, RedisResult};
use std::env;

use crate::{
    engine::{EventStatus, LightTask},
    models::NewEvent,
    parser::{ParsableEvent, Task},
    schema::events,
};
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
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
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
pub fn insert_event_into_db(
    conn: &mut PgConnection,
    event: ParsableEvent,
) -> Result<i32, AnyError> {
    // let client_result = create_postgres_client();
    // if let Err(e) = client_result {
    //     eprintln!("Failed to connect to postgres {}", e);
    //     eprintln!("exiting...");
    //     std::process::exit(1);
    // }
    // let mut client = client_result.unwrap();

    // let event_name = event.name;
    // let event_description = event.description;
    // let event_trigger = event.trigger;
    // use crate::schema::events::dsl::*;
    // let new_event = crate::models::NewEvent {
    //     name: &event_name,
    //     description: &event_description,
    //     trigger: &event_trigger,
    // };
    // let result = client.query_one(
    //     "INSERT INTO events (name, description, trigger, status) VALUES ($1, $2, $3, $4) RETURNING uid",
    //     &[
    //         &event_name,
    //         &event_description,
    //         &event_trigger,
    //         &EventStatus::Created.to_string(),
    //     ],
    // )?;
    // println!("results: {:?}", result);
    // let event_uid: i32 = result.get("uid");
    // println!("uid: {:?}", event_uid);
    let new_event = crate::models::NewEvent {
        name: event.name.as_deref(),
        description: event.description.as_deref(),
        trigger: &event.trigger,
    };
    let event_uid = diesel::insert_into(events::table)
        .values(&new_event)
        .returning(events::uid)
        .get_result::<i32>(conn)?;
    Ok(event_uid)
}

pub fn insert_event_tasks_into_db(
    conn: &mut PgConnection,
    tasks: Vec<Task>,
    event_uid: i32,
) -> Result<(), AnyError> {
    // let client_result: std::result::Result<Client, Error> = create_postgres_client();
    // if let Err(e) = client_result {
    //     eprintln!("Failed to connect to postgres {}", e);
    //     eprintln!("exiting...");
    //     std::process::exit(1);
    // }
    // let mut client = client_result.unwrap();

    for task in tasks {
        let task_name = task.name.unwrap_or("None".to_string());
        let task_description = task.description.unwrap_or("None".to_string());
        let task_path = task.path;
        let task_on_failure = task.on_failure;
        let task_status = EventStatus::Created.to_string();

        diesel::insert_into(crate::schema::tasks::table)
            .values((
                crate::schema::tasks::event_uid.eq(event_uid),
                crate::schema::tasks::name.eq(task_name),
                crate::schema::tasks::description.eq(task_description),
                crate::schema::tasks::path.eq(task_path),
                crate::schema::tasks::on_failure.eq(task_on_failure),
                crate::schema::tasks::status.eq(task_status),
            ))
            .execute(conn)?; // TODO: check if this is the correct way to do it

        // client.execute(
        //     "INSERT INTO tasks (event_uid, name, description, path, on_failure, status) VALUES ($1, $2, $3, $4, $5, $6)",
        //     &[&event_uid, &task_name, &task_description, &task_path, &task_on_failure, &task_status],
        // )?;
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
