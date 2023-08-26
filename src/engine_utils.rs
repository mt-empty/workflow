use anyhow::Error as AnyError;
use ctrlc::set_handler;
use diesel::PgConnection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, str};
// use workflow::components::event::poll_events;
// use workflow::components::task::queue_processor;
use crate::models::{EngineStatus, NewEngine};
use crate::schema;
use crate::utils::establish_pg_connection;
// mod models;
// mod utils;

use diesel::prelude::*;

pub fn run_process<F>(process_name: &str, process_fn: F, engine_uid: i32) -> Result<(), AnyError>
where
    F: FnOnce(Arc<AtomicBool>, i32) -> Result<(), AnyError>,
{
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(e) = process_fn(running, engine_uid) {
        eprintln!("Failed to start {} process: {}", process_name, e);
        eprintln!("exiting...");
        std::process::exit(1);
    }
    println!("{} process stopped correctly", process_name);

    Ok(())
}

// pub fn run_task_process(engine_uid: i32) -> Result<(), AnyError> {
//     run_process("Task", queue_processor, engine_uid)
// }

// pub fn run_event_process(engine_uid: i32) -> Result<(), AnyError> {
//     run_process("Event", poll_events, engine_uid)
// }

pub fn handle_stop() -> Result<(), AnyError> {
    diesel::update(schema::engines::table)
        .set(schema::engines::stop_signal.eq(true))
        .execute(&mut establish_pg_connection())?;
    Ok(())
}

pub fn update_engine_status(
    conn: &mut PgConnection,
    engine_uid: i32,
    engine_status: EngineStatus,
) -> Result<(), diesel::result::Error> {
    use crate::schema::engines::dsl::*;

    diesel::update(engines)
        .filter(uid.eq(engine_uid))
        .set(status.eq(engine_status.to_string()))
        .execute(conn)?;

    Ok(())
}

pub fn create_new_engine_entry(
    conn: &mut PgConnection,
    name: &str,
    ip_address: &str,
) -> Result<i32, diesel::result::Error> {
    use crate::schema::engines::table as engines;
    use crate::schema::engines::uid as engine_uid;

    let new_engine = NewEngine { name, ip_address };

    //insert and return uid
    diesel::insert_into(engines)
        .values(&new_engine)
        .returning(engine_uid)
        .get_result::<i32>(conn)
}
